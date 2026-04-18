// SPDX-License-Identifier: MIT
//! MCP Watchdog — monitors registered MCP-server processes, auto-restarts
//! them with exponential back-off on failure.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchdogState {
    pub server_id: String,
    pub consecutive_failures: u32,
    pub next_retry_unix: i64,
    pub last_ok_unix: i64,
}

pub fn backoff_seconds(consecutive_failures: u32) -> u64 {
    let base: u64 = 2;
    base.saturating_pow(consecutive_failures.min(10)).min(600)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_starts_small() {
        assert_eq!(backoff_seconds(0), 1);
        assert_eq!(backoff_seconds(1), 2);
        assert_eq!(backoff_seconds(4), 16);
    }

    #[test]
    fn backoff_caps_at_600() {
        assert_eq!(backoff_seconds(100), 600);
    }
}
