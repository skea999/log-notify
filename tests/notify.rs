//! Notification queue, worker, retry, and dropped-log tests.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

mod common;

use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::TestDir;
use log_notify::config::Urgency;
use log_notify::notify::{DroppedLog, EventQueue, NotificationEvent, NotifyError, Sleep, Worker};

struct FakeNotifier {
    results: VecDeque<Result<(), NotifyError>>,
    calls: Arc<Mutex<usize>>,
}

impl FakeNotifier {
    fn new(results: Vec<Result<(), NotifyError>>) -> (Self, Arc<Mutex<usize>>) {
        let calls = Arc::new(Mutex::new(0));
        (
            Self {
                results: results.into(),
                calls: Arc::clone(&calls),
            },
            calls,
        )
    }
}

impl log_notify::notify::Notifier for FakeNotifier {
    fn notify(&mut self, _event: &NotificationEvent) -> Result<(), NotifyError> {
        *self.calls.lock().unwrap() += 1;
        self.results.pop_front().unwrap_or(Ok(()))
    }
}

struct FakeSleep {
    sleeps: Arc<Mutex<Vec<Duration>>>,
}

impl FakeSleep {
    fn new() -> (Self, Arc<Mutex<Vec<Duration>>>) {
        let sleeps = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                sleeps: Arc::clone(&sleeps),
            },
            sleeps,
        )
    }
}

impl Sleep for FakeSleep {
    fn sleep(&self, duration: Duration) {
        self.sleeps.lock().unwrap().push(duration);
    }
}

fn event(source: usize, body: &str) -> NotificationEvent {
    NotificationEvent {
        source,
        source_path: PathBuf::from(format!("/var/log/source-{source}.log")),
        summary: "summary".to_owned(),
        body: body.to_owned(),
        urgency: Urgency::Normal,
    }
}

fn dropped_log(dir: &TestDir) -> (Arc<DroppedLog>, PathBuf) {
    let path = dir.file("dropped.log");
    (Arc::new(DroppedLog::open(&path)), path)
}

fn read_log(path: &PathBuf) -> String {
    fs::read_to_string(path).expect("read dropped log")
}

type WorkerFixture = (
    Arc<EventQueue>,
    Worker,
    Arc<Mutex<usize>>,
    Arc<Mutex<Vec<Duration>>>,
    PathBuf,
);

fn spawn_worker(dir: &TestDir, results: Vec<Result<(), NotifyError>>) -> WorkerFixture {
    let (dropped, path) = dropped_log(dir);
    let queue = Arc::new(EventQueue::new(vec![4], Arc::clone(&dropped)));
    let (notifier, calls) = FakeNotifier::new(results);
    let (sleeper, sleeps) = FakeSleep::new();
    let worker = Worker::spawn(
        Arc::clone(&queue),
        Box::new(notifier),
        dropped,
        Arc::new(sleeper),
    );
    (queue, worker, calls, sleeps, path)
}

#[test]
fn queue_overflow_drops_oldest_for_the_source() {
    let dir = TestDir::new("notify");
    let (dropped, path) = dropped_log(&dir);
    let queue = EventQueue::new(vec![2], dropped);

    queue.push(event(0, "a"));
    queue.push(event(0, "b"));
    queue.push(event(0, "c"));

    assert_eq!(queue.len(), 2);
    assert_eq!(queue.pop().expect("b").body, "b");
    assert_eq!(queue.pop().expect("c").body, "c");

    let log = read_log(&path);
    assert!(log.contains("queue_full"), "{log}");
    assert!(log.contains("\ta\n"), "{log}");
}

#[test]
fn capacities_are_independent_per_source() {
    let dir = TestDir::new("notify");
    let (dropped, path) = dropped_log(&dir);
    let queue = EventQueue::new(vec![1, 1], dropped);

    queue.push(event(0, "a"));
    queue.push(event(0, "b"));
    queue.push(event(1, "c"));

    assert_eq!(queue.pop().expect("b").body, "b");
    assert_eq!(queue.pop().expect("c").body, "c");
    assert!(queue.is_empty());

    let log = read_log(&path);
    assert!(log.contains("queue_full"), "{log}");
    assert!(log.contains("\ta\n"), "{log}");
}

#[test]
fn worker_retries_transient_then_succeeds() {
    let dir = TestDir::new("notify");
    let (queue, mut worker, calls, sleeps, path) = spawn_worker(
        &dir,
        vec![Err(NotifyError::Transient("temporary".to_owned())), Ok(())],
    );

    queue.push(event(0, "hello"));
    worker.shutdown();

    assert_eq!(*calls.lock().unwrap(), 2);
    assert_eq!(*sleeps.lock().unwrap(), vec![Duration::from_secs(1)]);
    assert!(read_log(&path).is_empty());
}

#[test]
fn worker_drops_after_retries_are_exhausted() {
    let dir = TestDir::new("notify");
    let (queue, mut worker, calls, sleeps, path) = spawn_worker(
        &dir,
        vec![
            Err(NotifyError::Transient("one".to_owned())),
            Err(NotifyError::Transient("two".to_owned())),
            Err(NotifyError::Transient("three".to_owned())),
            Err(NotifyError::Transient("four".to_owned())),
        ],
    );

    queue.push(event(0, "hello"));
    worker.shutdown();

    assert_eq!(*calls.lock().unwrap(), 4);
    assert_eq!(
        *sleeps.lock().unwrap(),
        vec![
            Duration::from_secs(1),
            Duration::from_secs(2),
            Duration::from_secs(4)
        ]
    );
    let log = read_log(&path);
    assert!(log.contains("notify_failed"), "{log}");
}

#[test]
fn worker_drops_missing_service_without_retry() {
    let dir = TestDir::new("notify");
    let (queue, mut worker, calls, sleeps, path) = spawn_worker(
        &dir,
        vec![Err(NotifyError::NoService("no session bus".to_owned()))],
    );

    queue.push(event(0, "hello"));
    worker.shutdown();

    assert_eq!(*calls.lock().unwrap(), 1);
    assert!(sleeps.lock().unwrap().is_empty());
    let log = read_log(&path);
    assert!(log.contains("no_service"), "{log}");
}

#[test]
fn worker_shutdown_without_events_is_immediate() {
    let dir = TestDir::new("notify");
    let (queue, mut worker, calls, _sleeps, path) = spawn_worker(&dir, Vec::new());

    worker.shutdown();

    assert_eq!(*calls.lock().unwrap(), 0);
    assert!(read_log(&path).is_empty());
    assert!(queue.is_empty());
}
