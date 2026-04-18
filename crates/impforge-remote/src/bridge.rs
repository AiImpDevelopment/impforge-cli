// SPDX-License-Identifier: MIT
//! Bridge abstraction — one trait implemented per transport.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BridgeKind {
    Telegram,
    Signal,
    WhatsApp,
}

impl BridgeKind {
    pub fn display(self) -> &'static str {
        match self {
            BridgeKind::Telegram => "telegram",
            BridgeKind::Signal => "signal",
            BridgeKind::WhatsApp => "whatsapp",
        }
    }

    pub fn all() -> &'static [BridgeKind] {
        &[BridgeKind::Telegram, BridgeKind::Signal, BridgeKind::WhatsApp]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeMessage {
    pub sender: String,
    pub text: String,
    pub received_at_unix: i64,
    pub kind: BridgeKind,
}

/// A bridge transport — sends a single text reply back to the originator.
pub trait Bridge: Send + Sync {
    fn kind(&self) -> BridgeKind;
    fn is_configured(&self) -> bool;
    fn send_reply(&self, msg: &BridgeMessage, reply_text: &str) -> anyhow::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_display_is_lowercase() {
        assert_eq!(BridgeKind::Telegram.display(), "telegram");
        assert_eq!(BridgeKind::Signal.display(), "signal");
        assert_eq!(BridgeKind::WhatsApp.display(), "whatsapp");
    }

    #[test]
    fn all_returns_three_kinds() {
        assert_eq!(BridgeKind::all().len(), 3);
    }

    #[test]
    fn message_serializes_roundtrip() {
        let m = BridgeMessage {
            sender: "alice".to_string(),
            text: "template list".to_string(),
            received_at_unix: 1_700_000_000,
            kind: BridgeKind::Telegram,
        };
        let j = serde_json::to_string(&m).expect("serialize");
        let back: BridgeMessage = serde_json::from_str(&j).expect("deserialize");
        assert_eq!(m, back);
    }
}
