// SPDX-License-Identifier: MIT
//! Canonical error type for the impforge-cli workspace.

use thiserror::Error;

pub type CoreResult<T> = std::result::Result<T, CoreError>;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("invalid manifest: {0}")]
    InvalidManifest(String),

    #[error("template '{0}' not found")]
    TemplateNotFound(String),

    #[error("skill '{0}' not found")]
    SkillNotFound(String),

    #[error("validation failed: {0}")]
    Validation(String),

    #[error("path '{0}' rejected: {1}")]
    UnsafePath(String, String),

    #[error("crypto failure: {0}")]
    Crypto(String),

    #[error("feature '{0}' is only available in impforge-aiimp (Pro)")]
    ProOnly(String),

    #[error("network error: {0}")]
    Network(String),

    #[error("other: {0}")]
    Other(String),
}

impl CoreError {
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }

    pub fn invalid_manifest(msg: impl Into<String>) -> Self {
        Self::InvalidManifest(msg.into())
    }

    pub fn pro_only(feature: impl Into<String>) -> Self {
        Self::ProOnly(feature.into())
    }

    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }

    pub fn crypto(msg: impl Into<String>) -> Self {
        Self::Crypto(msg.into())
    }
}
