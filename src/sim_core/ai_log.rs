//! Pure logic for AI decision/state logging.
//!
//! Contains the snapshot diff used by transition detection and the
//! `ThrashTracker` that decides when an ant has been changing state
//! fast enough to warrant dumping a ring buffer.
//!
//! Thrash rule: an ant is considered "thrashing" when it accumulates
//! more than one state change within a single 1-second window for
//! two consecutive windows. After a dump, a cooldown suppresses
//! repeated dumps for the same ant.

use crate::components::ant::{AntJob, AntState};

/// Minimal view of an ant's AI-relevant state.
///
/// Used both as the `prev` stored on `AiLogState` and as the payload
/// of transition records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AntSnapshot {
    pub state: AntState,
    pub job: AntJob,
    pub carrying: bool,
    pub on_surface: bool,
    /// Short label for the current nest task, if any (e.g. "W", "D", "H").
    pub nest_task: Option<&'static str>,
    /// Entity index of the committed combat target, when the ant is
    /// `Fighting`. `None` otherwise. Included so log readers can verify
    /// target-lock is stable across frames and confirm Fighting dwell.
    pub target: Option<u32>,
}

/// Difference between two snapshots, if any field changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransitionRecord {
    pub from: AntSnapshot,
    pub to: AntSnapshot,
}

/// Returns `Some(TransitionRecord)` when any tracked field changed.
pub fn diff(prev: AntSnapshot, curr: AntSnapshot) -> Option<TransitionRecord> {
    if prev == curr {
        None
    } else {
        Some(TransitionRecord { from: prev, to: curr })
    }
}

/// Duration of each counting window in seconds.
pub const THRASH_BUCKET_SECS: f32 = 1.0;

/// Consecutive high-rate windows required to trigger a dump.
pub const THRASH_TRIGGER_WINDOWS: u8 = 2;

/// Cooldown after a dump before the same ant can dump again.
pub const THRASH_COOLDOWN_SECS: f32 = 5.0;

/// Per-ant state for detecting thrashing.
///
/// The tracker is advanced once per frame via [`ThrashTracker::tick`].
#[derive(Debug, Clone, Copy, Default)]
pub struct ThrashTracker {
    /// State changes observed in the currently open window.
    pub changes_in_window: u8,
    /// `elapsed` at which the current window started.
    pub window_start: f32,
    /// Consecutive windows with `> 1` change.
    pub high_rate_windows: u8,
    /// If `> now`, dumps are suppressed for this ant.
    pub cooldown_until: f32,
}

/// Outcome of a single tick of the tracker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThrashResult {
    /// True if this tick closed the current 1s window.
    pub window_closed: bool,
    /// True if the caller should emit a ring-buffer dump for this ant.
    pub should_dump: bool,
}

impl ThrashTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that a state change occurred this frame.
    pub fn record_change(&mut self) {
        self.changes_in_window = self.changes_in_window.saturating_add(1);
    }

    /// Advance the tracker using current sim `elapsed` time.
    ///
    /// Closes the window if `elapsed - window_start >= THRASH_BUCKET_SECS`
    /// and evaluates whether a dump should fire.
    pub fn tick(&mut self, elapsed: f32) -> ThrashResult {
        let mut result = ThrashResult { window_closed: false, should_dump: false };
        if elapsed - self.window_start < THRASH_BUCKET_SECS {
            return result;
        }

        result.window_closed = true;
        if self.changes_in_window > 1 {
            self.high_rate_windows = self.high_rate_windows.saturating_add(1);
        } else {
            self.high_rate_windows = 0;
        }

        if self.high_rate_windows >= THRASH_TRIGGER_WINDOWS && elapsed >= self.cooldown_until {
            result.should_dump = true;
            self.cooldown_until = elapsed + THRASH_COOLDOWN_SECS;
            self.high_rate_windows = 0;
        }

        self.changes_in_window = 0;
        self.window_start = elapsed;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(state: AntState) -> AntSnapshot {
        AntSnapshot {
            state,
            job: AntJob::Forager,
            carrying: false,
            on_surface: true,
            nest_task: None,
            target: None,
        }
    }

    #[test]
    fn diff_returns_none_for_unchanged_snapshot() {
        let s = snap(AntState::Foraging);
        assert_eq!(diff(s, s), None);
    }

    #[test]
    fn diff_returns_some_when_state_changes() {
        let a = snap(AntState::Foraging);
        let b = snap(AntState::Returning);
        let d = diff(a, b).expect("expected diff");
        assert_eq!(d.from.state, AntState::Foraging);
        assert_eq!(d.to.state, AntState::Returning);
    }

    #[test]
    fn diff_returns_some_when_carrying_changes() {
        let a = snap(AntState::Foraging);
        let b = AntSnapshot { carrying: true, ..a };
        assert!(diff(a, b).is_some());
    }

    #[test]
    fn thrash_requires_two_consecutive_high_rate_windows() {
        let mut t = ThrashTracker::new();

        t.record_change();
        t.record_change();
        let r1 = t.tick(1.0);
        assert!(r1.window_closed);
        assert!(!r1.should_dump, "one high-rate window should not trigger");

        t.record_change();
        t.record_change();
        let r2 = t.tick(2.0);
        assert!(r2.window_closed);
        assert!(r2.should_dump, "second consecutive high-rate window should dump");
    }

    #[test]
    fn thrash_resets_on_calm_window() {
        let mut t = ThrashTracker::new();

        t.record_change();
        t.record_change();
        t.tick(1.0);

        let r_calm = t.tick(2.0);
        assert!(r_calm.window_closed);
        assert!(!r_calm.should_dump);
        assert_eq!(t.high_rate_windows, 0);

        t.record_change();
        t.record_change();
        let r3 = t.tick(3.0);
        assert!(r3.window_closed);
        assert!(!r3.should_dump, "calm window should have reset the streak");
    }

    #[test]
    fn thrash_single_change_per_window_is_not_thrashing() {
        let mut t = ThrashTracker::new();
        for i in 1..=5 {
            t.record_change();
            let r = t.tick(i as f32);
            assert!(r.window_closed);
            assert!(!r.should_dump);
        }
    }

    #[test]
    fn thrash_cooldown_suppresses_back_to_back_dumps() {
        let mut t = ThrashTracker::new();

        t.record_change();
        t.record_change();
        t.tick(1.0);
        t.record_change();
        t.record_change();
        let fire = t.tick(2.0);
        assert!(fire.should_dump);

        t.record_change();
        t.record_change();
        t.tick(3.0);
        t.record_change();
        t.record_change();
        let suppressed = t.tick(4.0);
        assert!(
            !suppressed.should_dump,
            "cooldown should suppress a second dump within {}s",
            THRASH_COOLDOWN_SECS
        );
    }

    #[test]
    fn thrash_tick_before_window_closes_is_noop() {
        let mut t = ThrashTracker::new();
        t.record_change();
        t.record_change();
        let r = t.tick(0.5);
        assert!(!r.window_closed);
        assert!(!r.should_dump);
        assert_eq!(t.changes_in_window, 2, "changes should persist until window closes");
    }
}
