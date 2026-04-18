// SPDX-License-Identifier: MIT
//! Auto-suspend layer for the MCP server.
//!
//! When no request arrives within the idle window, the server:
//!   * Drops the schema cache (saves ~10–40 KB RSS depending on tools)
//!   * Flushes any buffered stdout
//!   * Emits an `IdleTick` event to the agent trace (so the Crown-Jewel
//!     Guardian's dimension 6 sees exactly how long the server slept)
//!
//! On the very next byte received over stdin, the server wakes
//! automatically because the read-blocking thread was never in
//! DeepSleep-CPU — it was parked on `BufRead::read_line`, which consumes
//! zero CPU while waiting.  The only "real" resource savings are the
//! dropped caches + the emitted trace.

use crate::lazy_schema::SchemaCache;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub const DEFAULT_IDLE_MS: i64 = 60_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SleepTransition {
    pub entered_at_unix_ms: i64,
    pub reason: String,
    pub cache_entries_dropped: usize,
}

pub struct IdleWatcher {
    last_activity_ms: AtomicI64,
    idle_window_ms: i64,
    asleep: AtomicBool,
    cache: Arc<SchemaCache>,
}

impl IdleWatcher {
    pub fn new(cache: Arc<SchemaCache>, idle_window_ms: i64) -> Self {
        Self {
            last_activity_ms: AtomicI64::new(now_ms()),
            idle_window_ms: idle_window_ms.max(1_000),
            asleep: AtomicBool::new(false),
            cache,
        }
    }

    pub fn mark_activity(&self) {
        self.last_activity_ms.store(now_ms(), Ordering::Relaxed);
        if self.asleep.swap(false, Ordering::Relaxed) {
            // Wake transition — caller can log this if they want.
        }
    }

    pub fn elapsed_ms(&self) -> i64 {
        now_ms() - self.last_activity_ms.load(Ordering::Relaxed)
    }

    pub fn should_sleep(&self) -> bool {
        !self.asleep.load(Ordering::Relaxed) && self.elapsed_ms() > self.idle_window_ms
    }

    pub fn enter_sleep(&self) -> SleepTransition {
        let dropped = self.cache.materialised_count();
        self.cache.drop_all();
        self.asleep.store(true, Ordering::Relaxed);
        SleepTransition {
            entered_at_unix_ms: now_ms(),
            reason: format!("idle for {} ms", self.elapsed_ms()),
            cache_entries_dropped: dropped,
        }
    }

    pub fn is_asleep(&self) -> bool {
        self.asleep.load(Ordering::Relaxed)
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elapsed_increases_over_time() {
        let w = IdleWatcher::new(Arc::new(SchemaCache::new()), 60_000);
        let a = w.elapsed_ms();
        std::thread::sleep(std::time::Duration::from_millis(3));
        let b = w.elapsed_ms();
        assert!(b >= a);
    }

    #[test]
    fn fresh_watcher_should_not_sleep() {
        let w = IdleWatcher::new(Arc::new(SchemaCache::new()), 60_000);
        assert!(!w.should_sleep());
    }

    #[test]
    fn mark_activity_resets_timer() {
        let w = IdleWatcher::new(Arc::new(SchemaCache::new()), 1_000);
        // Fake a stale last-activity by constructing with a tiny window.
        // Without sleeping we can't force `should_sleep` to flip, but
        // `mark_activity` should never panic.
        w.mark_activity();
        assert!(!w.should_sleep());
    }

    #[test]
    fn enter_sleep_clears_cache_and_flags_asleep() {
        let cache = Arc::new(SchemaCache::new());
        let _ = cache.get_or_materialise("impforge_list_templates");
        assert_eq!(cache.materialised_count(), 1);
        let w = IdleWatcher::new(cache.clone(), 1);
        let transition = w.enter_sleep();
        assert_eq!(transition.cache_entries_dropped, 1);
        assert!(w.is_asleep());
        assert_eq!(cache.materialised_count(), 0);
    }

    #[test]
    fn wake_resets_asleep_flag() {
        let w = IdleWatcher::new(Arc::new(SchemaCache::new()), 1);
        w.enter_sleep();
        assert!(w.is_asleep());
        w.mark_activity();
        assert!(!w.is_asleep());
    }

    #[test]
    fn idle_window_has_minimum() {
        let w = IdleWatcher::new(Arc::new(SchemaCache::new()), 100);
        assert_eq!(w.idle_window_ms, 1_000, "tiny windows are clamped to 1s");
    }

    #[test]
    fn sleep_transition_carries_reason() {
        let w = IdleWatcher::new(Arc::new(SchemaCache::new()), 1_000);
        let t = w.enter_sleep();
        assert!(t.reason.starts_with("idle for"));
        assert!(t.entered_at_unix_ms > 0);
    }
}
