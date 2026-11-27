//! Per-source cooldown tests.

use std::time::{Duration, Instant};

use log_notify::cooldown::{Cooldown, CooldownSet};

#[test]
fn first_event_is_allowed() {
    let now = Instant::now();
    let mut cooldown = Cooldown::from_secs(60);

    assert!(cooldown.should_emit(now));
}

#[test]
fn second_event_inside_window_is_suppressed() {
    let start = Instant::now();
    let mut cooldown = Cooldown::from_secs(60);
    assert!(cooldown.should_emit(start));

    assert!(!cooldown.should_emit(start + Duration::from_secs(59)));
}

#[test]
fn event_at_window_boundary_is_allowed() {
    let start = Instant::now();
    let mut cooldown = Cooldown::from_secs(60);
    assert!(cooldown.should_emit(start));

    assert!(cooldown.should_emit(start + Duration::from_secs(60)));
}

#[test]
fn zero_window_is_disabled() {
    let start = Instant::now();
    let mut cooldown = Cooldown::from_secs(0);

    assert!(cooldown.should_emit(start));
    assert!(cooldown.should_emit(start));
    assert!(cooldown.should_emit(start + Duration::from_secs(1)));
    assert!(!cooldown.is_enabled());
}

#[test]
fn sources_have_independent_windows() {
    let start = Instant::now();
    let mut set = CooldownSet::new([60, 0]);

    assert!(set.should_emit(0, start));
    assert!(set.should_emit(1, start));
    assert!(!set.should_emit(0, start + Duration::from_secs(1)));
    assert!(set.should_emit(1, start + Duration::from_secs(1)));
}

#[test]
fn unknown_source_is_suppressed() {
    let now = Instant::now();
    let mut set = CooldownSet::new([60]);

    assert!(!set.should_emit(1, now));
}
