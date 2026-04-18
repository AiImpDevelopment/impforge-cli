// SPDX-License-Identifier: MIT
//! Error types for impforge-universal.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum UniversalError {
    #[error("tool '{0}' not found in registry")]
    ToolNotFound(String),

    #[error("duplicate tool id '{0}' — registry collision")]
    DuplicateTool(String),

    #[error("invalid schema: {0}")]
    InvalidSchema(String),

    #[error("invalid arguments: {0}")]
    InvalidArgs(String),

    #[error("provider error: {0}")]
    Provider(String),

    #[error("consumer error: {0}")]
    Consumer(String),

    #[error("security gate denied call '{tool}': {reason}")]
    SecurityDenied { tool: String, reason: String },

    #[error("react parse error: {0}")]
    ReactParse(String),

    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("other: {0}")]
    Other(String),
}

pub type UniversalResult<T> = Result<T, UniversalError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_not_found_is_displayed() {
        let e = UniversalError::ToolNotFound("foo".to_string());
        assert!(e.to_string().contains("foo"));
    }

    #[test]
    fn security_denied_includes_tool_and_reason() {
        let e = UniversalError::SecurityDenied {
            tool: "filesystem:write_file".to_string(),
            reason: "write to /etc blocked".to_string(),
        };
        let s = e.to_string();
        assert!(s.contains("filesystem:write_file"));
        assert!(s.contains("blocked"));
    }

    #[test]
    fn serde_error_wraps_properly() {
        let json_err: Result<i32, _> = serde_json::from_str("not json");
        let wrapped: UniversalError = json_err.unwrap_err().into();
        assert!(matches!(wrapped, UniversalError::Serde(_)));
    }
}
