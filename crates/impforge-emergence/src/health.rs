// SPDX-License-Identifier: MIT
//! Health reporting types.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthState {
    Healthy,
    Degraded,
    Unhealthy,
}

impl HealthState {
    pub fn as_str(self) -> &'static str {
        match self {
            HealthState::Healthy => "healthy",
            HealthState::Degraded => "degraded",
            HealthState::Unhealthy => "unhealthy",
        }
    }

    pub fn is_healthy(self) -> bool {
        matches!(self, HealthState::Healthy)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthReport {
    pub state: HealthState,
    pub detail: String,
    pub last_heartbeat_unix: i64,
}

impl HealthReport {
    pub fn healthy(detail: impl Into<String>, now_unix: i64) -> Self {
        Self { state: HealthState::Healthy, detail: detail.into(), last_heartbeat_unix: now_unix }
    }

    pub fn degraded(detail: impl Into<String>, now_unix: i64) -> Self {
        Self { state: HealthState::Degraded, detail: detail.into(), last_heartbeat_unix: now_unix }
    }

    pub fn unhealthy(detail: impl Into<String>, now_unix: i64) -> Self {
        Self { state: HealthState::Unhealthy, detail: detail.into(), last_heartbeat_unix: now_unix }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_state_is_healthy() {
        assert!(HealthState::Healthy.is_healthy());
        assert!(!HealthState::Degraded.is_healthy());
        assert!(!HealthState::Unhealthy.is_healthy());
    }

    #[test]
    fn health_report_serializes() {
        let r = HealthReport::degraded("Ollama not responding", 1_700_000_000);
        let j = serde_json::to_string(&r).expect("serialize");
        assert!(j.contains("degraded"));
        assert!(j.contains("Ollama"));
    }
}
