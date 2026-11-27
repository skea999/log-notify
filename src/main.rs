//! `log-notify` CLI entry point.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use clap::Parser;
use signal_hook::consts::signal::{SIGHUP, SIGINT, SIGTERM};
use signal_hook::iterator::Signals;

use log_notify::config::{self, Config};
use log_notify::cooldown::CooldownSet;
use log_notify::lock::Lock;
use log_notify::notify::{
    self, DroppedLog, DryRunNotifier, EventQueue, NotificationEvent, Notifier, SystemSleep, Worker,
    ZbusNotifier,
};
use log_notify::watch::{TailedLine, Wake, Watcher};

#[derive(Debug, Parser)]
#[command(
    name = "log-notify",
    version,
    about = "Tail log files and emit freedesktop notifications"
)]
struct Cli {
    /// Config file. Defaults to /etc/log-notify/log-notify.toml, then
    /// ~/.config/log-notify/log-notify.toml.
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,

    /// Print matched events to stdout instead of notifying.
    #[arg(long)]
    dry_run: bool,

    /// Exit after the first match is processed.
    #[arg(long)]
    once: bool,

    /// Add tail-debug lines to stderr.
    #[arg(long)]
    verbose: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("log-notify: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    let config = config::load_from(cli.config.as_deref())?;
    let active: Vec<usize> = config
        .sources
        .iter()
        .enumerate()
        .filter(|(_, source)| source.enabled)
        .map(|(index, _)| index)
        .collect();
    if active.is_empty() {
        return Err("no enabled [[source]] entries".into());
    }

    let paths: Vec<PathBuf> = active
        .iter()
        .filter_map(|&index| config.sources.get(index))
        .map(|source| source.path.clone())
        .collect();
    let _lock = Lock::acquire_default()?;
    let mut watcher = Watcher::new(&paths, config.general.line_cap_bytes)?;
    install_signal_handlers(&watcher)?;
    let dropped = Arc::new(DroppedLog::open(&config.general.dropped_log));
    let mut cooldowns = CooldownSet::new(
        active
            .iter()
            .filter_map(|&index| config.sources.get(index))
            .map(|source| source.cooldown_secs),
    );

    let mut server = start_worker(cli, &config, &active, &dropped);
    let mut dry_run = DryRunNotifier;
    let mut zbus = ZbusNotifier::default();

    loop {
        print_warnings(&mut watcher, cli.verbose);
        let wake = watcher.wait()?;
        print_warnings(&mut watcher, cli.verbose);

        match wake {
            Wake::Shutdown => break,
            Wake::Idle => {}
            Wake::Lines(lines) => {
                for tailed in lines {
                    let Some(event) = matched_event(&config, &active, &mut cooldowns, &tailed)
                    else {
                        continue;
                    };
                    log_event(&event);

                    if cli.dry_run {
                        if let Err(error) = dry_run.notify(&event) {
                            eprintln!("log-notify: dry-run failed: {error}");
                        }
                    } else if cli.once {
                        notify::deliver_event(&mut zbus, dropped.as_ref(), &SystemSleep, event);
                    } else if let Some((queue, _)) = server.as_ref() {
                        queue.push(event);
                    }

                    if cli.once {
                        if let Some((_, mut worker)) = server.take() {
                            worker.shutdown();
                        }
                        return Ok(());
                    }
                }
            }
        }
    }

    if let Some((_, mut worker)) = server.take() {
        worker.shutdown();
    }
    Ok(())
}

fn install_signal_handlers(watcher: &Watcher) -> Result<(), Box<dyn std::error::Error>> {
    let mut signals = Signals::new([SIGTERM, SIGINT, SIGHUP])?;
    let shutdown = watcher.shutdown_handle();
    let _signal_thread = thread::spawn(move || {
        for signal in signals.forever() {
            match signal {
                SIGTERM | SIGINT => {
                    shutdown.trigger();
                    return;
                }
                SIGHUP => {
                    eprintln!("log-notify: SIGHUP ignored; restart to reload config");
                }
                _ => {}
            }
        }
    });
    Ok(())
}

fn start_worker(
    cli: &Cli,
    config: &Config,
    active: &[usize],
    dropped: &Arc<DroppedLog>,
) -> Option<(Arc<EventQueue>, Worker)> {
    if cli.dry_run || cli.once {
        return None;
    }
    let capacities: Vec<usize> = active
        .iter()
        .filter_map(|&index| config.sources.get(index))
        .map(|source| source.queue_capacity)
        .collect();
    let queue = Arc::new(EventQueue::new(capacities, Arc::clone(dropped)));
    let worker = Worker::spawn(
        Arc::clone(&queue),
        Box::new(ZbusNotifier::default()),
        Arc::clone(dropped),
        Arc::new(SystemSleep),
    );
    Some((queue, worker))
}

fn matched_event(
    config: &Config,
    active: &[usize],
    cooldowns: &mut CooldownSet,
    tailed: &TailedLine,
) -> Option<NotificationEvent> {
    let source_index = *active.get(tailed.source)?;
    let source = config.sources.get(source_index)?;
    let captures = source.compiled.captures(&tailed.line.text)?;
    if !cooldowns.should_emit(tailed.source, Instant::now()) {
        return None;
    }
    let rendered = source.compiled.render(&tailed.line.text, &captures);
    Some(NotificationEvent {
        source: tailed.source,
        source_path: source.path.clone(),
        summary: rendered.summary,
        body: rendered.body,
        urgency: source.urgency,
    })
}

fn print_warnings(watcher: &mut Watcher, verbose: bool) {
    for warning in watcher.take_warnings() {
        if verbose {
            eprintln!("log-notify: tail: {warning}");
        } else {
            eprintln!("log-notify: {warning}");
        }
    }
}

fn log_event(event: &NotificationEvent) {
    eprintln!(
        "log-notify: {}: {} [{:?}]",
        event.source_path.display(),
        event.summary,
        event.urgency
    );
}
