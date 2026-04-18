// SPDX-License-Identifier: MIT
//! impforge-aiimp-updater — the thinnest possible MIT companion crate.
//!
//! Single responsibility: fetch the latest signed `impforge-aiimp` binary
//! from <https://impforge.com/releases>, verify the SHA-256 checksum and
//! Ed25519 signature against the pinned maintainer public key, and write
//! it to the platform-appropriate install location.
//!
//! This crate contains **zero** proprietary ImpForge internals — it only
//! knows the public release API shape and the Ed25519 public key used to
//! verify downloads.  Source is MIT, safe to audit.

use clap::{Parser, Subcommand};

mod pubkey;
mod release;
mod verify;

#[derive(Debug, Parser)]
#[command(
    name = "impforge-aiimp-updater",
    about = "Version-check + SHA-256 + Ed25519-verified downloader for impforge-aiimp",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Check which version is currently installed.
    Check,
    /// List all published impforge-aiimp versions.
    List,
    /// Download + verify the latest release.
    Install,
    /// Download a specific version.
    InstallVersion { version: String },
    /// Print the pinned maintainer public key.
    Pubkey,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "impforge_aiimp_updater=info".into()),
        )
        .with_target(false)
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Check => release::check()?,
        Command::List => release::list()?,
        Command::Install => release::install_latest()?,
        Command::InstallVersion { version } => release::install(&version)?,
        Command::Pubkey => println!("{}", pubkey::MAINTAINER_ED25519_PUBLIC_HEX),
    }
    Ok(())
}
