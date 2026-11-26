# log-notify — Domain Vocabulary

Terms used consistently across code, docs, and conversations.

## Terms

**Source**
A configured log file plus its pattern, notification templates, urgency, and
cooldown. One `[[source]]` entry in `log-notify.toml`.
_Avoid_: feed, input, watcher

**Pattern**
A Rust `regex` expression applied to each complete log line from a source.
Compiled and validated at startup.
_Avoid_: filter, rule, matcher

**Match**
One complete line that satisfies a source's pattern. A match may be suppressed
by cooldown before it becomes a notification.
_Avoid_: hit, event, alert

**Event**
A match that reaches the notification stage. This is what `--dry-run` prints and
what stderr logging reports.
_Avoid_: alert, notification (reserve that for the D-Bus call)

**Tail offset**
The byte position of the next unread byte in a source file. At startup it is the
current file size; no history is read.
_Avoid_: cursor, pointer, position

**Partial line**
Bytes after the last newline that do not yet form a complete line. Buffered until
the newline arrives or the line cap is reached.
_Avoid_: fragment, chunk, incomplete record

**Line cap**
Maximum bytes considered per line before matching. Default `8192`. Longer lines
are truncated; the daemon must not panic on them.
_Avoid_: max line, limit

**Rotation**
Replacement or truncation of a log file. Two cases matter:

- **copytruncate:** the file shrinks below the last offset; reset the offset to 0.
- **inode change:** the path now points to a different file; reopen it from its
  current end.
_Avoid_: rollover, recycle

**Cooldown**
Per-source suppression window after a notification. Further matches from the
same source are ignored until the window expires. `0` disables suppression.
_Avoid_: rate limit, throttle, debounce

**Notification queue**
Bounded in-memory queue between the main loop and the notifier worker. Each
source has its own capacity; the worker drains in FIFO order.
_Avoid_: buffer, backlog

**Queue capacity**
Maximum pending notifications per source: `10` by default, `1..=100` allowed.
When full, the oldest event for that source is dropped and logged.
_Avoid_: backlog size, buffer length

**Dropped event**
A notification that could not be delivered or queued. It is written to the
dropped-event log with a reason: `queue_full`, `notify_failed`, or `no_service`.
_Avoid_: lost notification, discard

**Dropped-event log**
File configured by `general.dropped_log`, default `/var/log/log-notify.log`. If
unwritable, the daemon falls back to stderr after one warning.
_Avoid_: error log, dead-letter file

**Notifier worker**
Dedicated thread that owns the zbus 6.5 connection and sends notifications. The
main loop never calls D-Bus, so a slow or hung notification daemon cannot stop
log tailing.
_Avoid_: sender, dispatcher

**Urgency**
Notification importance: `low`, `normal`, or `critical`. Maps to the freedesktop
urgency hint (`0`, `1`, `2`). Critical notifications never expire.
_Avoid_: priority, severity

**Dry run**
Mode that runs the full pipeline but prints events to stdout instead of calling
D-Bus. Used for pattern testing and integration tests.
_Avoid_: test mode, simulation

**Single instance**
Advisory lock at `$XDG_RUNTIME_DIR/log-notify.lock`. A second daemon exits with
an error instead of double-notifying.
_Avoid_: singleton, mutex

**Session service**
A user-scoped service managed by the desktop session or service manager. It runs
with the user's D-Bus session and log read permissions.

**Default config**
The planned in-repo `config/log-notify.toml`. It targets FeralOS security-stack
log paths; other deployments replace it. It is also the config installed as
`/etc/log-notify/log-notify.toml` on FeralOS.

**Downstream**
FeralOS packaging and service integration. It belongs to the FeralOS repositories,
not this repo.
