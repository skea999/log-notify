//! Per-source notification cooldown.
//!
//! The window starts after a successful enqueue. Suppressed matches are
//! discarded, not queued. Time is injected so tests never sleep.

use std::time::{Duration, Instant};

/// Cooldown window for one source.
#[derive(Debug, Clone)]
pub struct Cooldown {
    window: Duration,
    last_emitted: Option<Instant>,
}

impl Cooldown {
    /// Build a cooldown from a seconds value. `0` disables suppression.
    #[must_use]
    pub fn from_secs(seconds: u64) -> Self {
        Self {
            window: Duration::from_secs(seconds),
            last_emitted: None,
        }
    }

    /// Whether this cooldown is active.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        !self.window.is_zero()
    }

    /// Return `true` when an event may be emitted, and record the emission.
    ///
    /// A window of zero always allows. Otherwise the first event and any event
    /// at or after `last + window` are allowed.
    pub fn should_emit(&mut self, now: Instant) -> bool {
        if self.is_enabled()
            && let Some(last) = self.last_emitted
            && now.saturating_duration_since(last) < self.window
        {
            return false;
        }
        self.last_emitted = Some(now);
        true
    }
}

/// Cooldowns for every configured source, indexed like the source list.
#[derive(Debug, Clone)]
pub struct CooldownSet {
    entries: Vec<Cooldown>,
}

impl CooldownSet {
    /// Build one cooldown per configured source.
    #[must_use]
    pub fn new(seconds: impl IntoIterator<Item = u64>) -> Self {
        Self {
            entries: seconds.into_iter().map(Cooldown::from_secs).collect(),
        }
    }

    /// Whether the source may emit at `now`. Unknown indices are suppressed.
    pub fn should_emit(&mut self, source: usize, now: Instant) -> bool {
        self.entries
            .get_mut(source)
            .is_some_and(|cooldown| cooldown.should_emit(now))
    }
}
