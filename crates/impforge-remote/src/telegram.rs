// SPDX-License-Identifier: MIT
//! Telegram Bot API bridge — the lowest-friction of the three because
//! Telegram offers a first-class HTTP bot API (no app install needed).
//!
//! Configuration lives in the env var `IMPFORGE_TELEGRAM_TOKEN`, matching
//! the standard Telegram Bot API convention.

use crate::bridge::{Bridge, BridgeKind, BridgeMessage};
use serde::Serialize;

const TELEGRAM_API: &str = "https://api.telegram.org";

pub struct TelegramBridge {
    token: Option<String>,
}

impl Default for TelegramBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl TelegramBridge {
    pub fn new() -> Self {
        Self {
            token: std::env::var("IMPFORGE_TELEGRAM_TOKEN").ok(),
        }
    }

    pub fn with_token(token: impl Into<String>) -> Self {
        Self { token: Some(token.into()) }
    }
}

#[derive(Serialize)]
struct SendMessageBody<'a> {
    chat_id: &'a str,
    text: &'a str,
    parse_mode: &'a str,
}

impl Bridge for TelegramBridge {
    fn kind(&self) -> BridgeKind { BridgeKind::Telegram }

    fn is_configured(&self) -> bool {
        self.token.as_deref().map(|t| !t.is_empty()).unwrap_or(false)
    }

    fn send_reply(&self, msg: &BridgeMessage, reply_text: &str) -> anyhow::Result<()> {
        let Some(token) = &self.token else {
            anyhow::bail!("TELEGRAM_TOKEN not set — run `IMPFORGE_TELEGRAM_TOKEN=… impforge-cli remote start`");
        };
        let url = format!("{TELEGRAM_API}/bot{token}/sendMessage");
        let body = SendMessageBody {
            chat_id: &msg.sender,
            text: reply_text,
            parse_mode: "Markdown",
        };
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        let resp = client.post(&url).json(&body).send()?;
        if !resp.status().is_success() {
            anyhow::bail!("telegram send failed: HTTP {}", resp.status());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_is_telegram() {
        let b = TelegramBridge::with_token("abc");
        assert_eq!(b.kind(), BridgeKind::Telegram);
    }

    #[test]
    fn is_configured_with_token() {
        let b = TelegramBridge::with_token("abc");
        assert!(b.is_configured());
    }

    #[test]
    fn is_not_configured_without_token() {
        let b = TelegramBridge { token: None };
        assert!(!b.is_configured());
    }

    #[test]
    fn is_not_configured_with_empty_token() {
        let b = TelegramBridge::with_token("");
        assert!(!b.is_configured());
    }

    #[test]
    fn send_reply_requires_token() {
        let b = TelegramBridge { token: None };
        let msg = BridgeMessage {
            sender: "123".to_string(),
            text: "x".to_string(),
            received_at_unix: 0,
            kind: BridgeKind::Telegram,
        };
        assert!(b.send_reply(&msg, "reply").is_err());
    }
}
