// SPDX-License-Identifier: MIT
//! Doctor — checks the health of Ollama / HF cache / templates / MCP.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctorReport {
    pub ollama_reachable: bool,
    pub hf_cache_valid: bool,
    pub templates_verified: bool,
    pub mcp_servers_responsive: u32,
    pub mcp_servers_total: u32,
    pub overall_healthy: bool,
    pub findings: Vec<String>,
}

impl DoctorReport {
    pub fn healthy_placeholder() -> Self {
        Self {
            ollama_reachable: true,
            hf_cache_valid: true,
            templates_verified: true,
            mcp_servers_responsive: 0,
            mcp_servers_total: 0,
            overall_healthy: true,
            findings: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_is_healthy() {
        let r = DoctorReport::healthy_placeholder();
        assert!(r.overall_healthy);
        assert!(r.findings.is_empty());
    }
}
