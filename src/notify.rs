//! Bounded notification queue, worker, dropped-event log, and zbus backend.

use std::collections::{HashMap, VecDeque};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use thiserror::Error;
use zbus::Variant;

use crate::config::Urgency;

/// Retry delays after a transient D-Bus failure.
pub const RETRY_DELAYS: [Duration; 3] = [
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(4),
];
/// Timeout for one zbus method call.
pub const METHOD_TIMEOUT: Duration = Duration::from_secs(5);

/// One notification to deliver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationEvent {
    /// Source index.
    pub source: usize,
    /// Source log path.
    pub source_path: PathBuf,
    /// Notification title.
    pub summary: String,
    /// Notification body.
    pub body: String,
    /// Notification urgency.
    pub urgency: Urgency,
}

/// Why an event was dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropReason {
    /// The per-source queue was full.
    QueueFull,
    /// No notification service is available.
    NoService,
    /// Retries were exhausted.
    NotifyFailed,
}

impl DropReason {
    /// Stable reason string for the dropped-event log.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::QueueFull => "queue_full",
            Self::NoService => "no_service",
            Self::NotifyFailed => "notify_failed",
        }
    }
}

/// One dropped notification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DroppedEvent {
    /// The event that was dropped.
    pub event: NotificationEvent,
    /// Drop reason.
    pub reason: DropReason,
    /// Optional backend detail.
    pub detail: Option<String>,
}

/// Notification backend failure.
#[derive(Debug, Error)]
pub enum NotifyError {
    /// The notification service is not available; do not retry.
    #[error("notification service unavailable: {0}")]
    NoService(String),
    /// A transient failure; retry with backoff.
    #[error("notification failed: {0}")]
    Transient(String),
}

/// Deliver one notification.
pub trait Notifier: Send {
    /// Send the event. `NoService` is terminal; `Transient` is retried.
    fn notify(&mut self, event: &NotificationEvent) -> Result<(), NotifyError>;
}

/// Sleep abstraction so retry tests never wait.
pub trait Sleep: Send + Sync {
    /// Sleep for the requested duration.
    fn sleep(&self, duration: Duration);
}

/// Real sleeper used by the daemon.
#[derive(Debug, Default)]
pub struct SystemSleep;

impl Sleep for SystemSleep {
    fn sleep(&self, duration: Duration) {
        thread::sleep(duration);
    }
}

/// Append-only log for dropped notifications.
#[derive(Debug)]
pub struct DroppedLog {
    writer: Mutex<DroppedWriter>,
}

#[derive(Debug)]
enum DroppedWriter {
    File(BufWriter<File>),
    Stderr,
}

impl DroppedLog {
    /// Open the configured dropped-event log. On failure, warn once and use
    /// stderr; the daemon keeps running.
    #[must_use]
    pub fn open(path: &Path) -> Self {
        match OpenOptions::new().create(true).append(true).open(path) {
            Ok(file) => Self {
                writer: Mutex::new(DroppedWriter::File(BufWriter::new(file))),
            },
            Err(error) => {
                eprintln!(
                    "log-notify: cannot open dropped log {}: {error}; using stderr",
                    path.display()
                );
                Self::stderr()
            }
        }
    }

    /// Log dropped events to stderr.
    #[must_use]
    pub fn stderr() -> Self {
        Self {
            writer: Mutex::new(DroppedWriter::Stderr),
        }
    }

    /// Write one dropped event.
    pub fn log(&self, dropped: &DroppedEvent) {
        let line = format_dropped(dropped);
        let mut writer = self
            .writer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let result = match &mut *writer {
            DroppedWriter::File(file) => {
                file.write_all(line.as_bytes()).and_then(|()| file.flush())
            }
            DroppedWriter::Stderr => {
                eprint!("{line}");
                Ok(())
            }
        };
        if let Err(error) = result {
            eprintln!("log-notify: cannot write dropped event: {error}");
        }
    }
}

/// Bounded per-source queue shared by the main loop and the worker.
#[derive(Debug)]
pub struct EventQueue {
    inner: Mutex<QueueInner>,
    ready: Condvar,
    capacities: Vec<usize>,
    dropped: Arc<DroppedLog>,
}

#[derive(Debug)]
struct QueueInner {
    events: VecDeque<NotificationEvent>,
    per_source: Vec<usize>,
    shutdown: bool,
}

impl EventQueue {
    /// Create a queue with one capacity per source.
    #[must_use]
    pub fn new(capacities: Vec<usize>, dropped: Arc<DroppedLog>) -> Self {
        let per_source = vec![0; capacities.len()];
        Self {
            inner: Mutex::new(QueueInner {
                events: VecDeque::new(),
                per_source,
                shutdown: false,
            }),
            ready: Condvar::new(),
            capacities,
            dropped,
        }
    }

    /// Enqueue an event. Overflow drops the oldest event of the same source
    /// and writes it to the dropped-event log.
    pub fn push(&self, event: NotificationEvent) {
        let source = event.source;
        let capacity = self.capacities.get(source).copied().unwrap_or(0);
        if capacity == 0 {
            self.dropped.log(&DroppedEvent {
                event,
                reason: DropReason::QueueFull,
                detail: Some("unknown source index".to_owned()),
            });
            return;
        }

        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if inner.shutdown {
            return;
        }

        let count = inner.per_source.get(source).copied().unwrap_or(0);
        if count >= capacity
            && let Some(position) = inner
                .events
                .iter()
                .position(|queued| queued.source == source)
            && let Some(dropped) = inner.events.remove(position)
        {
            if let Some(current) = inner.per_source.get_mut(source) {
                *current = current.saturating_sub(1);
            }
            self.dropped.log(&DroppedEvent {
                event: dropped,
                reason: DropReason::QueueFull,
                detail: None,
            });
        }

        inner.events.push_back(event);
        if let Some(current) = inner.per_source.get_mut(source) {
            *current = current.saturating_add(1);
        }
        self.ready.notify_one();
    }

    /// Pop the oldest event, blocking until one exists or shutdown is requested.
    pub fn pop(&self) -> Option<NotificationEvent> {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        while inner.events.is_empty() && !inner.shutdown {
            inner = self
                .ready
                .wait(inner)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
        let event = inner.events.pop_front()?;
        if let Some(current) = inner.per_source.get_mut(event.source) {
            *current = current.saturating_sub(1);
        }
        Some(event)
    }

    /// Wake the worker and stop accepting events.
    pub fn shutdown(&self) {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        inner.shutdown = true;
        self.ready.notify_all();
    }

    /// Current number of queued events.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .events
            .len()
    }

    /// Whether the queue is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Notifier worker handle.
#[derive(Debug)]
pub struct Worker {
    queue: Arc<EventQueue>,
    handle: Option<JoinHandle<()>>,
}

impl Worker {
    /// Spawn the notification worker.
    pub fn spawn(
        queue: Arc<EventQueue>,
        notifier: Box<dyn Notifier>,
        dropped: Arc<DroppedLog>,
        sleeper: Arc<dyn Sleep>,
    ) -> Self {
        let worker_queue = Arc::clone(&queue);
        let handle = thread::spawn(move || {
            run_worker(
                worker_queue.as_ref(),
                notifier,
                dropped.as_ref(),
                sleeper.as_ref(),
            );
        });
        Self {
            queue,
            handle: Some(handle),
        }
    }

    /// Stop accepting events and join the worker.
    pub fn shutdown(&mut self) {
        self.queue.shutdown();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Dry-run notifier: print one stable event line, never touch D-Bus.
#[derive(Debug, Default)]
pub struct DryRunNotifier;

impl Notifier for DryRunNotifier {
    fn notify(&mut self, event: &NotificationEvent) -> Result<(), NotifyError> {
        println!(
            "[{}] {}: {}",
            event.source_path.display(),
            event.summary,
            one_line(&event.body)
        );
        Ok(())
    }
}

/// zbus 6.5 notification backend.
#[derive(Debug, Default)]
pub struct ZbusNotifier {
    connection: Option<zbus::Connection>,
}

impl Notifier for ZbusNotifier {
    fn notify(&mut self, event: &NotificationEvent) -> Result<(), NotifyError> {
        if self.connection.is_none() {
            self.connection = Some(connect()?);
        }
        let Some(connection) = self.connection.as_ref() else {
            return Err(NotifyError::NoService("session bus unavailable".to_owned()));
        };
        let result = send(connection, event);
        if result.is_err() {
            self.connection = None;
        }
        result
    }
}

#[allow(clippy::too_many_arguments)]
#[zbus::proxy(
    interface = "org.freedesktop.Notifications",
    default_service = "org.freedesktop.Notifications",
    default_path = "/org/freedesktop/Notifications"
)]
trait Notifications {
    fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: &[&str],
        hints: HashMap<&str, Variant<'_>>,
        expire_timeout: i32,
    ) -> zbus::Result<u32>;
}

fn connect() -> Result<zbus::Connection, NotifyError> {
    zbus::block_on(async {
        zbus::connection::Builder::session()
            .method_timeout(METHOD_TIMEOUT)
            .build()
            .await
    })
    .map_err(|error| classify(&error))
}

fn send(connection: &zbus::Connection, event: &NotificationEvent) -> Result<(), NotifyError> {
    let proxy =
        zbus::block_on(NotificationsProxy::new(connection)).map_err(|error| classify(&error))?;
    let mut hints = HashMap::new();
    hints.insert("urgency", Variant::U8(event.urgency.hint()));
    zbus::block_on(proxy.notify(
        "log-notify",
        0,
        "",
        &event.summary,
        &event.body,
        &[],
        hints,
        event.urgency.expire_timeout(),
    ))
    .map_err(|error| classify(&error))?;
    Ok(())
}

fn run_worker(
    queue: &EventQueue,
    mut notifier: Box<dyn Notifier>,
    dropped: &DroppedLog,
    sleeper: &dyn Sleep,
) {
    while let Some(event) = queue.pop() {
        process_event(notifier.as_mut(), dropped, sleeper, event);
    }
}

fn process_event(
    notifier: &mut dyn Notifier,
    dropped: &DroppedLog,
    sleeper: &dyn Sleep,
    event: NotificationEvent,
) {
    let mut retry = 0;
    loop {
        match notifier.notify(&event) {
            Ok(()) => return,
            Err(NotifyError::NoService(detail)) => {
                dropped.log(&DroppedEvent {
                    event,
                    reason: DropReason::NoService,
                    detail: Some(detail),
                });
                return;
            }
            Err(NotifyError::Transient(detail)) => {
                let Some(delay) = RETRY_DELAYS.get(retry) else {
                    dropped.log(&DroppedEvent {
                        event,
                        reason: DropReason::NotifyFailed,
                        detail: Some(detail),
                    });
                    return;
                };
                sleeper.sleep(*delay);
                retry += 1;
            }
        }
    }
}

fn classify(error: &zbus::Error) -> NotifyError {
    let text = error.to_string();
    if text.contains("ServiceUnknown")
        || text.contains("NameHasNoOwner")
        || text.contains("org.freedesktop.DBus.Error.ServiceUnknown")
    {
        NotifyError::NoService(text)
    } else {
        NotifyError::Transient(text)
    }
}

fn format_dropped(dropped: &DroppedEvent) -> String {
    let timestamp = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "unknown".to_owned());
    let detail = dropped.detail.as_deref().map_or_else(String::new, one_line);
    format!(
        "{timestamp}\t{}\t{}\t{detail}\t{}\t{}\n",
        dropped.event.source_path.display(),
        dropped.reason.as_str(),
        one_line(&dropped.event.summary),
        one_line(&dropped.event.body)
    )
}

fn one_line(text: &str) -> String {
    text.chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect()
}
