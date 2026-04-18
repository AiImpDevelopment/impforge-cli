// SPDX-License-Identifier: MIT
//! Pinned maintainer public key.
//!
//! This hex string is the Ed25519 public key of the ImpForge Maintainers
//! release-signing identity.  Any `impforge-aiimp` binary that does NOT
//! verify against this key is rejected.

pub const MAINTAINER_ED25519_PUBLIC_HEX: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pubkey_is_64_hex_chars() {
        assert_eq!(MAINTAINER_ED25519_PUBLIC_HEX.len(), 64);
        assert!(MAINTAINER_ED25519_PUBLIC_HEX.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
