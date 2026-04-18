// SPDX-License-Identifier: MIT
//! Auto-generated MCP client registrations.
//!
//! Given a supported AI-coding client id, this module produces the exact
//! JSON config fragment the user should paste.

use impforge_core::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClientId {
    ClaudeCode,
    Cursor,
    Lovable,
    Bolt,
    Windsurf,
    Zed,
    Continue,
    Cline,
}

impl ClientId {
    pub fn all() -> &'static [ClientId] {
        &[
            ClientId::ClaudeCode,
            ClientId::Cursor,
            ClientId::Lovable,
            ClientId::Bolt,
            ClientId::Windsurf,
            ClientId::Zed,
            ClientId::Continue,
            ClientId::Cline,
        ]
    }

    pub fn display(self) -> &'static str {
        match self {
            ClientId::ClaudeCode => "claude-code",
            ClientId::Cursor => "cursor",
            ClientId::Lovable => "lovable",
            ClientId::Bolt => "bolt",
            ClientId::Windsurf => "windsurf",
            ClientId::Zed => "zed",
            ClientId::Continue => "continue",
            ClientId::Cline => "cline",
        }
    }

    pub fn parse(s: &str) -> CoreResult<Self> {
        Ok(match s {
            "claude-code" => ClientId::ClaudeCode,
            "cursor" => ClientId::Cursor,
            "lovable" => ClientId::Lovable,
            "bolt" => ClientId::Bolt,
            "windsurf" => ClientId::Windsurf,
            "zed" => ClientId::Zed,
            "continue" => ClientId::Continue,
            "cline" => ClientId::Cline,
            other => {
                return Err(CoreError::validation(format!(
                    "unknown MCP client '{other}' — supported: {}",
                    Self::all()
                        .iter()
                        .map(|c| c.display())
                        .collect::<Vec<_>>()
                        .join(", ")
                )));
            }
        })
    }
}

/// Produce the JSON config snippet a user should paste into their client.
pub fn config_snippet(client: ClientId) -> String {
    let server_config = serde_json::json!({
        "command": "impforge-cli",
        "args": ["mcp", "serve"],
        "env": {}
    });

    match client {
        ClientId::ClaudeCode | ClientId::Cursor | ClientId::Windsurf | ClientId::Cline => {
            serde_json::to_string_pretty(&serde_json::json!({
                "mcpServers": {
                    "impforge": server_config
                }
            }))
            .unwrap_or_default()
        }
        ClientId::Lovable | ClientId::Bolt => {
            serde_json::to_string_pretty(&serde_json::json!({
                "mcp": {
                    "impforge": {
                        "type": "stdio",
                        "command": "impforge-cli",
                        "args": ["mcp", "serve"]
                    }
                }
            }))
            .unwrap_or_default()
        }
        ClientId::Zed => {
            serde_json::to_string_pretty(&serde_json::json!({
                "context_servers": {
                    "impforge": {
                        "command": {
                            "path": "impforge-cli",
                            "args": ["mcp", "serve"]
                        }
                    }
                }
            }))
            .unwrap_or_default()
        }
        ClientId::Continue => {
            serde_json::to_string_pretty(&serde_json::json!({
                "mcpServers": [{
                    "name": "impforge",
                    "transport": { "type": "stdio", "command": "impforge-cli", "args": ["mcp", "serve"] }
                }]
            }))
            .unwrap_or_default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_all_clients() {
        for client in ClientId::all() {
            let parsed = ClientId::parse(client.display()).expect("parse");
            assert_eq!(parsed, *client);
        }
    }

    #[test]
    fn unknown_client_rejected() {
        assert!(ClientId::parse("ghost-ide").is_err());
    }

    #[test]
    fn every_client_produces_snippet() {
        for client in ClientId::all() {
            let snippet = config_snippet(*client);
            assert!(!snippet.is_empty(), "empty for {client:?}");
            assert!(snippet.contains("impforge"));
        }
    }
}
