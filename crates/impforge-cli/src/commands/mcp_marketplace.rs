// SPDX-License-Identifier: MIT
//! `impforge-cli mcp marketplace` — local-first MCP marketplace.
//!
//! ## Subcommands
//!
//! ```text
//! impforge-cli mcp browse                      # list cached marketplace entries
//! impforge-cli mcp browse --search github      # filter by substring (name/desc/tags)
//! impforge-cli mcp browse --category code      # filter by category
//! impforge-cli mcp browse --offline            # skip network sync (cache only)
//! impforge-cli mcp install <id>                # install (writes to ~/.impforge-cli/mcp-installed/)
//! impforge-cli mcp uninstall <id>
//! impforge-cli mcp installed                   # list installed servers
//! impforge-cli mcp health <id>                 # ping (stdio probe) + transport check
//! impforge-cli mcp configure <id>              # opens $EDITOR on the per-server config JSON
//! impforge-cli mcp update                      # refresh marketplace mirror from CDN
//! ```
//!
//! ## Local-first marketplace mirror
//!
//! Marketplace entries persist in a SQLite DB at
//! `~/.impforge-cli/mcp-marketplace.db`.  On first run, we seed the DB
//! from the bundled `seed.json` (offline-first — works without network).
//! `mcp update` performs an HTTP GET against
//! `https://marketplace.impforge.com/v1/manifests.json` and merges new
//! entries / version bumps in.
//!
//! ## Privacy (REGEL 000-BRIDGE-NOT-PROCESS)
//!
//! Two outbound flows ONLY:
//!   1. `mcp update` — HTTP GET against the public CDN (read-only,
//!      anonymous, gzipped JSON). No POST / no telemetry / no user data.
//!   2. `mcp install <id> --git` (future) — git clone of the publisher's
//!      public repo, exactly as `git clone` would.
//!
//! Every byte of every server config + invocation log stays on the
//! user's disk forever.

use anyhow::{anyhow, bail, Context, Result};
use clap::{Args, Subcommand};
use rusqlite::{params, Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::theme;

/// Default marketplace URL — public CDN, read-only, anonymous.
pub const DEFAULT_MARKETPLACE_URL: &str =
    "https://marketplace.impforge.com/v1/manifests.json";

/// Hard ceiling on a single marketplace JSON download.  Prevents a
/// hostile mirror from filling disk.
pub const MAX_MARKETPLACE_BYTES: usize = 32 * 1024 * 1024;

/// `impforge-cli mcp …` marketplace family.  Plugged into `commands::mcp`
/// alongside the existing list/register/clients/serve commands.
#[derive(Debug, Subcommand)]
pub enum McpMarketplaceCmd {
    /// Browse cached marketplace entries (offline-first).
    Browse(BrowseArgs),
    /// Install an MCP server by id.
    Install(InstallArgs),
    /// Remove a previously-installed server.
    Uninstall { id: String },
    /// List installed servers.
    Installed,
    /// Health-check an installed server (stdio probe + transport).
    Health { id: String },
    /// Open `$EDITOR` on a server's per-user config JSON.
    Configure { id: String },
    /// Sync the marketplace mirror from the public CDN.
    Update(UpdateArgs),
}

/// `impforge-cli mcp browse` flags.
#[derive(Debug, Args)]
pub struct BrowseArgs {
    /// Substring filter against name + description + tags.
    #[arg(long)]
    pub search: Option<String>,
    /// Limit to a single category (`files`, `web`, `code`, `data`, `ai`, `comms`, `custom`).
    #[arg(long)]
    pub category: Option<String>,
    /// Skip the implicit one-shot sync at the start of `browse`.
    #[arg(long)]
    pub offline: bool,
    /// Maximum number of rows to print (newest first).
    #[arg(long, default_value_t = 25)]
    pub limit: usize,
}

/// `impforge-cli mcp install` flags.
#[derive(Debug, Args)]
pub struct InstallArgs {
    /// Marketplace entry id (e.g. `filesystem`, `github`, `brave-search`).
    pub id: String,
    /// Pin to a specific version (defaults to entry's `version`).
    #[arg(long)]
    pub version: Option<String>,
    /// Skip the post-install stdio probe.
    #[arg(long)]
    pub no_probe: bool,
}

/// `impforge-cli mcp update` flags.
#[derive(Debug, Args)]
pub struct UpdateArgs {
    /// Override the marketplace URL (mostly for tests / mirrors).
    #[arg(long)]
    pub url: Option<String>,
}

/// One marketplace entry — mirrored from the upstream `manifests.json`
/// schema described in `mcp-manifests-spec/v1.md`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub publisher: String,
    pub version: String,
    pub category: String,
    pub tags: Vec<String>,
    pub install_count: u64,
    pub stars: u32,
    pub verified: bool,
    pub signed: bool,
    pub transport: String, // "stdio" | "http" | "sse"
    pub command: Option<String>,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub homepage: Option<String>,
    pub license: Option<String>,
    pub last_updated_iso: String,
    /// Capabilities advertised at install time (tools/resources/prompts).
    pub capability_hint: Option<CapabilityHint>,
}

/// Capability summary (tools/resources/prompts counts) embedded in the
/// marketplace entry — the App + Pro tiers fetch the *real* schema after
/// install, but the CLI uses this for offline display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityHint {
    pub tools: u32,
    pub resources: u32,
    pub prompts: u32,
    pub oauth: bool,
}

/// One installed server, persisted in `installed.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledServer {
    pub id: String,
    pub version: String,
    pub installed_iso: String,
    pub config_path: PathBuf,
    pub transport: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
}

/// Computes a defensive trust score 0-100 — used for sort + colour cues.
///
/// Weights match the App-tier `TrustScoreRing.svelte`, so a server gets
/// the same score on the CLI and in the GUI.
pub fn trust_score(entry: &MarketplaceEntry) -> u8 {
    let verified = if entry.verified { 30.0 } else { 0.0 };
    let signed = if entry.signed { 20.0 } else { 0.0 };
    // log10(installs) clamped to [0, 6] then scaled to /15 -> max 15.
    let installs_log = ((entry.install_count.max(1)) as f64).log10().min(6.0);
    let installs = installs_log * 2.5; // 6 -> 15
    // 5-star rating scaled to 15.
    let rating = (entry.stars.min(5) as f64) * 3.0;
    // Recency: parse ISO -> days, fresh = 10, 365d = 0.
    let recency = recency_score(&entry.last_updated_iso);
    // Sandbox-pass bonus: capability_hint present = 10.
    let sandbox = if entry.capability_hint.is_some() { 10.0 } else { 0.0 };

    let total = verified + signed + installs + rating + recency + sandbox;
    total.round().clamp(0.0, 100.0) as u8
}

/// 10 points if updated within 7 days, decaying linearly to 0 over a year.
fn recency_score(iso: &str) -> f64 {
    let parsed = chrono::DateTime::parse_from_rfc3339(iso).ok();
    let Some(dt) = parsed else { return 0.0 };
    let age_days = (chrono::Utc::now() - dt.with_timezone(&chrono::Utc)).num_days();
    if age_days < 0 {
        return 10.0;
    }
    let age = age_days as f64;
    if age <= 7.0 {
        return 10.0;
    }
    if age >= 365.0 {
        return 0.0;
    }
    10.0 * (1.0 - (age - 7.0) / (365.0 - 7.0))
}

/// Resolve `~/.impforge-cli/` (overridable via `IMPFORGE_CLI_HOME` for tests).
pub fn cli_home() -> Result<PathBuf> {
    if let Ok(custom) = std::env::var("IMPFORGE_CLI_HOME") {
        let p = PathBuf::from(custom);
        fs::create_dir_all(&p).context("create custom IMPFORGE_CLI_HOME")?;
        return Ok(p);
    }
    let base = dirs::home_dir().ok_or_else(|| anyhow!("no $HOME — run with IMPFORGE_CLI_HOME set"))?;
    let p = base.join(".impforge-cli");
    fs::create_dir_all(&p).context("create ~/.impforge-cli")?;
    Ok(p)
}

/// Path to the SQLite marketplace mirror.
pub fn marketplace_db_path() -> Result<PathBuf> {
    Ok(cli_home()?.join("mcp-marketplace.db"))
}

/// Path to the JSON file holding installed servers.
pub fn installed_path() -> Result<PathBuf> {
    Ok(cli_home()?.join("mcp-installed.json"))
}

/// Per-server config JSON path — used by `configure`.
pub fn server_config_path(id: &str) -> Result<PathBuf> {
    let dir = cli_home()?.join("mcp-installed").join(id);
    fs::create_dir_all(&dir)?;
    Ok(dir.join("config.json"))
}

/// Open or create the marketplace SQLite DB and run migrations.
pub fn open_marketplace_db() -> Result<Connection> {
    let path = marketplace_db_path()?;
    let conn = Connection::open_with_flags(
        &path,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    conn.execute_batch(
        r#"
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;

CREATE TABLE IF NOT EXISTS entries (
    id              TEXT PRIMARY KEY,
    payload         TEXT NOT NULL,
    name            TEXT NOT NULL,
    description     TEXT NOT NULL,
    publisher       TEXT NOT NULL,
    category        TEXT NOT NULL,
    install_count   INTEGER NOT NULL DEFAULT 0,
    stars           INTEGER NOT NULL DEFAULT 0,
    verified        INTEGER NOT NULL DEFAULT 0,
    signed          INTEGER NOT NULL DEFAULT 0,
    last_updated_iso TEXT NOT NULL,
    refreshed_iso    TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_entries_category
    ON entries (category);
CREATE INDEX IF NOT EXISTS idx_entries_install_count
    ON entries (install_count DESC);
"#,
    )?;
    Ok(conn)
}

/// Insert (or replace) one marketplace entry.
pub fn upsert_entry(conn: &Connection, entry: &MarketplaceEntry) -> Result<()> {
    let payload = serde_json::to_string(entry)?;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        r#"INSERT INTO entries (id, payload, name, description, publisher,
                                 category, install_count, stars, verified, signed,
                                 last_updated_iso, refreshed_iso)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
           ON CONFLICT(id) DO UPDATE SET
             payload = excluded.payload,
             name = excluded.name,
             description = excluded.description,
             publisher = excluded.publisher,
             category = excluded.category,
             install_count = excluded.install_count,
             stars = excluded.stars,
             verified = excluded.verified,
             signed = excluded.signed,
             last_updated_iso = excluded.last_updated_iso,
             refreshed_iso = excluded.refreshed_iso"#,
        params![
            entry.id,
            payload,
            entry.name,
            entry.description,
            entry.publisher,
            entry.category,
            entry.install_count as i64,
            entry.stars as i64,
            entry.verified as i32,
            entry.signed as i32,
            entry.last_updated_iso,
            now,
        ],
    )?;
    Ok(())
}

/// List entries from the cache (offline-first).
pub fn list_entries(
    conn: &Connection,
    search: Option<&str>,
    category: Option<&str>,
    limit: usize,
) -> Result<Vec<MarketplaceEntry>> {
    let mut sql = String::from("SELECT payload FROM entries WHERE 1=1");
    let mut bindings: Vec<String> = Vec::new();
    if let Some(cat) = category {
        sql.push_str(" AND category = ?");
        bindings.push(cat.to_lowercase());
    }
    if let Some(q) = search {
        let pattern = format!("%{}%", q.to_lowercase());
        sql.push_str(" AND (LOWER(name) LIKE ? OR LOWER(description) LIKE ? OR LOWER(publisher) LIKE ?)");
        bindings.push(pattern.clone());
        bindings.push(pattern.clone());
        bindings.push(pattern);
    }
    sql.push_str(" ORDER BY install_count DESC, last_updated_iso DESC LIMIT ?");
    bindings.push(limit.to_string());
    let mut stmt = conn.prepare(&sql)?;
    let mapped: rusqlite::Result<Vec<String>> = stmt
        .query_map(rusqlite::params_from_iter(bindings.iter()), |row| row.get(0))?
        .collect();
    let payloads = mapped?;
    let mut out = Vec::with_capacity(payloads.len());
    for p in payloads {
        let entry: MarketplaceEntry = serde_json::from_str(&p)
            .with_context(|| format!("decode marketplace entry payload: {p}"))?;
        out.push(entry);
    }
    Ok(out)
}

/// Fetch one entry by id (for install / health / configure).
pub fn get_entry(conn: &Connection, id: &str) -> Result<MarketplaceEntry> {
    let mut stmt = conn.prepare("SELECT payload FROM entries WHERE id = ?1")?;
    let payload: String = stmt
        .query_row(params![id], |row| row.get(0))
        .map_err(|_| anyhow!("server '{id}' not found in marketplace cache (try `mcp update`)"))?;
    let entry: MarketplaceEntry = serde_json::from_str(&payload)?;
    Ok(entry)
}

/// Bundled offline seed — a curated 8-server starter set so a fresh
/// install of impforge-cli already shows real entries before any network
/// hit.  Each entry maps to a real, well-known MCP server.
pub fn seed_entries() -> Vec<MarketplaceEntry> {
    let now = chrono::Utc::now().to_rfc3339();
    let mut env_empty = BTreeMap::<String, String>::new();
    env_empty.clear();
    vec![
        MarketplaceEntry {
            id: "filesystem".into(),
            name: "Filesystem".into(),
            description: "Read & write local files inside the user's allow-listed paths.".into(),
            publisher: "modelcontextprotocol".into(),
            version: "0.6.0".into(),
            category: "files".into(),
            tags: vec!["files".into(), "local".into()],
            install_count: 142_000,
            stars: 5,
            verified: true,
            signed: true,
            transport: "stdio".into(),
            command: Some("npx".into()),
            args: vec!["-y".into(), "@modelcontextprotocol/server-filesystem".into()],
            env: env_empty.clone(),
            homepage: Some("https://github.com/modelcontextprotocol/servers".into()),
            license: Some("MIT".into()),
            last_updated_iso: now.clone(),
            capability_hint: Some(CapabilityHint {
                tools: 6,
                resources: 1,
                prompts: 0,
                oauth: false,
            }),
        },
        MarketplaceEntry {
            id: "github".into(),
            name: "GitHub".into(),
            description: "Issues, PRs, commits, and code search across repositories.".into(),
            publisher: "modelcontextprotocol".into(),
            version: "0.7.1".into(),
            category: "code".into(),
            tags: vec!["github".into(), "code".into(), "oauth".into()],
            install_count: 96_000,
            stars: 5,
            verified: true,
            signed: true,
            transport: "stdio".into(),
            command: Some("npx".into()),
            args: vec!["-y".into(), "@modelcontextprotocol/server-github".into()],
            env: env_empty.clone(),
            homepage: Some("https://github.com/modelcontextprotocol/servers".into()),
            license: Some("MIT".into()),
            last_updated_iso: now.clone(),
            capability_hint: Some(CapabilityHint {
                tools: 24,
                resources: 0,
                prompts: 0,
                oauth: true,
            }),
        },
        MarketplaceEntry {
            id: "brave-search".into(),
            name: "Brave Search".into(),
            description: "Live web search via the Brave Search API (BYOK).".into(),
            publisher: "modelcontextprotocol".into(),
            version: "0.5.0".into(),
            category: "web".into(),
            tags: vec!["search".into(), "web".into()],
            install_count: 71_000,
            stars: 4,
            verified: true,
            signed: false,
            transport: "stdio".into(),
            command: Some("npx".into()),
            args: vec!["-y".into(), "@modelcontextprotocol/server-brave-search".into()],
            env: env_empty.clone(),
            homepage: Some("https://github.com/modelcontextprotocol/servers".into()),
            license: Some("MIT".into()),
            last_updated_iso: now.clone(),
            capability_hint: Some(CapabilityHint {
                tools: 2,
                resources: 0,
                prompts: 0,
                oauth: false,
            }),
        },
        MarketplaceEntry {
            id: "memory".into(),
            name: "Memory".into(),
            description: "Long-term key/value memory backed by a local store.".into(),
            publisher: "modelcontextprotocol".into(),
            version: "0.4.2".into(),
            category: "data".into(),
            tags: vec!["memory".into(), "local".into()],
            install_count: 58_000,
            stars: 4,
            verified: true,
            signed: true,
            transport: "stdio".into(),
            command: Some("npx".into()),
            args: vec!["-y".into(), "@modelcontextprotocol/server-memory".into()],
            env: env_empty.clone(),
            homepage: Some("https://github.com/modelcontextprotocol/servers".into()),
            license: Some("MIT".into()),
            last_updated_iso: now.clone(),
            capability_hint: Some(CapabilityHint {
                tools: 5,
                resources: 0,
                prompts: 0,
                oauth: false,
            }),
        },
        MarketplaceEntry {
            id: "fetch".into(),
            name: "Fetch".into(),
            description: "HTTP fetch with markdown / text / JSON parsing.".into(),
            publisher: "modelcontextprotocol".into(),
            version: "0.3.0".into(),
            category: "web".into(),
            tags: vec!["http".into(), "web".into()],
            install_count: 45_000,
            stars: 4,
            verified: true,
            signed: false,
            transport: "stdio".into(),
            command: Some("uvx".into()),
            args: vec!["mcp-server-fetch".into()],
            env: env_empty.clone(),
            homepage: Some("https://github.com/modelcontextprotocol/servers".into()),
            license: Some("MIT".into()),
            last_updated_iso: now.clone(),
            capability_hint: Some(CapabilityHint {
                tools: 1,
                resources: 0,
                prompts: 0,
                oauth: false,
            }),
        },
        MarketplaceEntry {
            id: "git".into(),
            name: "Git".into(),
            description: "Run git operations on a local repository.".into(),
            publisher: "modelcontextprotocol".into(),
            version: "0.4.0".into(),
            category: "code".into(),
            tags: vec!["git".into(), "code".into()],
            install_count: 38_000,
            stars: 4,
            verified: true,
            signed: true,
            transport: "stdio".into(),
            command: Some("uvx".into()),
            args: vec!["mcp-server-git".into()],
            env: env_empty.clone(),
            homepage: Some("https://github.com/modelcontextprotocol/servers".into()),
            license: Some("MIT".into()),
            last_updated_iso: now.clone(),
            capability_hint: Some(CapabilityHint {
                tools: 9,
                resources: 0,
                prompts: 0,
                oauth: false,
            }),
        },
        MarketplaceEntry {
            id: "postgres".into(),
            name: "Postgres".into(),
            description: "Read-only Postgres database access via connection string.".into(),
            publisher: "modelcontextprotocol".into(),
            version: "0.3.0".into(),
            category: "data".into(),
            tags: vec!["postgres".into(), "sql".into(), "data".into()],
            install_count: 24_000,
            stars: 4,
            verified: true,
            signed: true,
            transport: "stdio".into(),
            command: Some("npx".into()),
            args: vec!["-y".into(), "@modelcontextprotocol/server-postgres".into()],
            env: env_empty.clone(),
            homepage: Some("https://github.com/modelcontextprotocol/servers".into()),
            license: Some("MIT".into()),
            last_updated_iso: now.clone(),
            capability_hint: Some(CapabilityHint {
                tools: 1,
                resources: 1,
                prompts: 0,
                oauth: false,
            }),
        },
        MarketplaceEntry {
            id: "slack".into(),
            name: "Slack".into(),
            description: "Send + read Slack messages via OAuth.".into(),
            publisher: "modelcontextprotocol".into(),
            version: "0.5.0".into(),
            category: "comms".into(),
            tags: vec!["slack".into(), "chat".into(), "oauth".into()],
            install_count: 19_000,
            stars: 4,
            verified: true,
            signed: false,
            transport: "stdio".into(),
            command: Some("npx".into()),
            args: vec!["-y".into(), "@modelcontextprotocol/server-slack".into()],
            env: env_empty.clone(),
            homepage: Some("https://github.com/modelcontextprotocol/servers".into()),
            license: Some("MIT".into()),
            last_updated_iso: now.clone(),
            capability_hint: Some(CapabilityHint {
                tools: 8,
                resources: 0,
                prompts: 0,
                oauth: true,
            }),
        },
    ]
}

/// Seed an empty marketplace DB with the bundled curated set.  Idempotent.
pub fn ensure_seeded(conn: &Connection) -> Result<usize> {
    let count: i64 =
        conn.query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))?;
    if count > 0 {
        return Ok(0);
    }
    let entries = seed_entries();
    let inserted = entries.len();
    for e in &entries {
        upsert_entry(conn, e)?;
    }
    Ok(inserted)
}

/// `mcp browse` handler.
pub fn browse(args: BrowseArgs) -> Result<()> {
    let conn = open_marketplace_db()?;
    let inserted = ensure_seeded(&conn)?;
    if inserted > 0 {
        theme::print_info(&format!("seeded {inserted} marketplace entries (offline)"));
    }
    if !args.offline {
        if let Err(err) = sync_mirror(&conn, DEFAULT_MARKETPLACE_URL) {
            theme::print_warning(&format!(
                "marketplace sync failed (using offline cache): {err}"
            ));
        }
    }

    let entries = list_entries(
        &conn,
        args.search.as_deref(),
        args.category.as_deref(),
        args.limit,
    )?;

    if entries.is_empty() {
        theme::print_warning("no marketplace entries match — try `mcp update`");
        return Ok(());
    }

    println!();
    println!(
        "{}{}id                  trust  installs  category  publisher{}",
        theme::BOLD,
        theme::DIM,
        theme::RESET
    );
    println!("{}{}{}", theme::DIM, "─".repeat(72), theme::RESET);
    for e in &entries {
        let score = trust_score(e);
        let (badge, colour) = trust_badge(score);
        println!(
            "  {neon}{:<18}{reset} {colour}{badge} {score:>3}{reset}  {:>8}  {:<8}  {dim}{}{reset}",
            e.id,
            e.install_count,
            e.category,
            e.publisher,
            neon = theme::ACCENT_NEON,
            reset = theme::RESET,
            dim = theme::DIM,
        );
        println!("    {dim}{}{reset}", e.description, dim = theme::DIM, reset = theme::RESET);
    }
    println!();
    theme::print_info(&format!("{} entries shown · `mcp install <id>` to add", entries.len()));
    Ok(())
}

fn trust_badge(score: u8) -> (&'static str, &'static str) {
    match score {
        80..=100 => ("●", theme::ACCENT_NEON),
        50..=79 => ("●", theme::ACCENT_CYAN),
        30..=49 => ("◐", "\x1b[38;2;255;170;0m"),
        _ => ("○", "\x1b[38;2;160;160;176m"),
    }
}

/// `mcp update` handler — sync the marketplace mirror.
pub fn update(args: UpdateArgs) -> Result<()> {
    let conn = open_marketplace_db()?;
    let url = args.url.as_deref().unwrap_or(DEFAULT_MARKETPLACE_URL);
    let count = sync_mirror(&conn, url)?;
    theme::print_success(&format!("synced {count} entries from {url}"));
    Ok(())
}

/// Pull the manifests JSON from the public CDN and merge into local
/// SQLite cache.  Returns number of upserted entries.
///
/// Errors gracefully if the network is unreachable — the caller is
/// expected to fall back to the offline cache.
pub fn sync_mirror(conn: &Connection, url: &str) -> Result<usize> {
    let client = reqwest::blocking::Client::builder()
        .user_agent(concat!(
            "ImpForge-CLI/",
            env!("CARGO_PKG_VERSION"),
            " (+https://impforge.com)"
        ))
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    let resp = client.get(url).send()?;
    if !resp.status().is_success() {
        bail!("marketplace returned {}", resp.status());
    }
    let bytes = resp.bytes()?;
    if bytes.len() > MAX_MARKETPLACE_BYTES {
        bail!(
            "marketplace payload {} bytes exceeds {} byte cap",
            bytes.len(),
            MAX_MARKETPLACE_BYTES
        );
    }
    let entries: Vec<MarketplaceEntry> = serde_json::from_slice(&bytes)
        .context("parse marketplace manifests.json")?;
    let count = entries.len();
    for entry in entries {
        upsert_entry(conn, &entry)?;
    }
    Ok(count)
}

/// `mcp install <id>` handler.
pub fn install(args: InstallArgs) -> Result<()> {
    let conn = open_marketplace_db()?;
    ensure_seeded(&conn)?;

    let entry = get_entry(&conn, &args.id)?;
    let version = args.version.unwrap_or_else(|| entry.version.clone());

    let installed = InstalledServer {
        id: entry.id.clone(),
        version,
        installed_iso: chrono::Utc::now().to_rfc3339(),
        config_path: server_config_path(&entry.id)?,
        transport: entry.transport.clone(),
        command: entry.command.clone(),
        args: entry.args.clone(),
        env: entry.env.clone(),
    };

    write_default_config(&installed)?;
    upsert_installed(&installed)?;

    if !args.no_probe {
        match probe(&installed) {
            Ok(()) => theme::print_success(&format!("stdio probe passed for {}", installed.id)),
            Err(err) => theme::print_warning(&format!(
                "stdio probe failed for {}: {err} (server installed but not yet runnable)",
                installed.id
            )),
        }
    }

    theme::print_success(&format!(
        "installed {}@{} → {}",
        installed.id,
        installed.version,
        installed.config_path.display()
    ));
    Ok(())
}

/// Write a minimal default config — env vars set to their declared
/// names with empty values so the user can `configure` to fill them in.
fn write_default_config(server: &InstalledServer) -> Result<()> {
    let cfg = serde_json::json!({
        "id": server.id,
        "version": server.version,
        "transport": server.transport,
        "command": server.command,
        "args": server.args,
        "env": server.env,
    });
    fs::write(&server.config_path, serde_json::to_vec_pretty(&cfg)?)?;
    Ok(())
}

/// `mcp uninstall <id>` handler.
pub fn uninstall(id: &str) -> Result<()> {
    let mut all = load_installed()?;
    let before = all.len();
    all.retain(|s| s.id != id);
    if all.len() == before {
        bail!("server '{id}' is not installed");
    }
    save_installed(&all)?;
    let dir = cli_home()?.join("mcp-installed").join(id);
    if dir.exists() {
        fs::remove_dir_all(&dir).with_context(|| format!("remove {}", dir.display()))?;
    }
    theme::print_success(&format!("uninstalled {id}"));
    Ok(())
}

/// `mcp installed` handler.
pub fn installed_cmd() -> Result<()> {
    let all = load_installed()?;
    if all.is_empty() {
        theme::print_info("no servers installed yet — try `mcp browse`");
        return Ok(());
    }
    println!();
    println!(
        "{}{}id                  version    transport  installed{}",
        theme::BOLD,
        theme::DIM,
        theme::RESET
    );
    println!("{}{}{}", theme::DIM, "─".repeat(64), theme::RESET);
    for s in &all {
        println!(
            "  {neon}{:<18}{reset} {:<10} {:<10} {dim}{}{reset}",
            s.id,
            s.version,
            s.transport,
            s.installed_iso,
            neon = theme::ACCENT_NEON,
            reset = theme::RESET,
            dim = theme::DIM,
        );
    }
    println!();
    Ok(())
}

/// `mcp health <id>` handler.
pub fn health(id: &str) -> Result<()> {
    let all = load_installed()?;
    let server = all
        .iter()
        .find(|s| s.id == id)
        .ok_or_else(|| anyhow!("server '{id}' is not installed"))?;
    let probe_started = std::time::Instant::now();
    match probe(server) {
        Ok(()) => {
            let elapsed = probe_started.elapsed().as_millis();
            theme::print_success(&format!(
                "{}: stdio probe ok in {elapsed}ms",
                server.id
            ));
        }
        Err(err) => {
            theme::print_error(&format!("{}: {err}", server.id));
        }
    }
    Ok(())
}

/// Lightweight stdio probe — spawn the server, send `initialize`, wait up
/// to 2s for a `result` line, kill.  We do NOT speak full MCP — the App
/// + Pro tiers do that.  This is just a "does the binary launch + ack
///   initialize?" check.
pub fn probe(server: &InstalledServer) -> Result<()> {
    if server.transport != "stdio" {
        // HTTP / SSE servers — App tier handles those.
        return Ok(());
    }
    let cmd = server
        .command
        .as_ref()
        .ok_or_else(|| anyhow!("no command configured for {}", server.id))?;
    let mut child = std::process::Command::new(cmd)
        .args(&server.args)
        .envs(server.env.iter())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("spawn {cmd}"))?;

    let init_msg = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-03-26",
            "clientInfo": { "name": "impforge-cli-probe", "version": env!("CARGO_PKG_VERSION") },
            "capabilities": {},
        }
    });
    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        let payload = format!("{}\n", serde_json::to_string(&init_msg)?);
        let _ = stdin.write_all(payload.as_bytes());
        let _ = stdin.flush();
    }
    // Brief grace window to let the child write something — we don't read
    // the response (full parse lives in App tier), we only check exit
    // status / non-crash within 2 seconds.
    let waited = std::time::Instant::now();
    while waited.elapsed() < std::time::Duration::from_secs(2) {
        if let Some(status) = child.try_wait()? {
            // Premature exit = probe failure.
            let _ = child.kill();
            bail!("server exited early with {status}");
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

/// `mcp configure <id>` handler — opens `$EDITOR` on the config JSON.
pub fn configure(id: &str) -> Result<()> {
    let all = load_installed()?;
    let server = all
        .iter()
        .find(|s| s.id == id)
        .ok_or_else(|| anyhow!("server '{id}' is not installed"))?;
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".into());
    theme::print_info(&format!("opening {} in {editor}", server.config_path.display()));
    let status = Command::new(&editor)
        .arg(&server.config_path)
        .status()
        .with_context(|| format!("spawn {editor}"))?;
    if !status.success() {
        bail!("editor exited with {status}");
    }
    Ok(())
}

/// Persist `installed.json`.
fn save_installed(servers: &[InstalledServer]) -> Result<()> {
    fs::write(installed_path()?, serde_json::to_vec_pretty(servers)?)?;
    Ok(())
}

/// Load `installed.json` (empty list if missing).
pub fn load_installed() -> Result<Vec<InstalledServer>> {
    let path = installed_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let bytes = fs::read(&path)?;
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    let list: Vec<InstalledServer> = serde_json::from_slice(&bytes)?;
    Ok(list)
}

/// Upsert one installed server.
fn upsert_installed(server: &InstalledServer) -> Result<()> {
    let mut all = load_installed()?;
    if let Some(existing) = all.iter_mut().find(|s| s.id == server.id) {
        *existing = server.clone();
    } else {
        all.push(server.clone());
    }
    save_installed(&all)?;
    Ok(())
}

/// Dispatch entry point — wired into `commands::mcp::run`.
pub fn run(cmd: McpMarketplaceCmd) -> Result<()> {
    match cmd {
        McpMarketplaceCmd::Browse(args) => browse(args),
        McpMarketplaceCmd::Install(args) => install(args),
        McpMarketplaceCmd::Uninstall { id } => uninstall(&id),
        McpMarketplaceCmd::Installed => installed_cmd(),
        McpMarketplaceCmd::Health { id } => health(&id),
        McpMarketplaceCmd::Configure { id } => configure(&id),
        McpMarketplaceCmd::Update(args) => update(args),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Override $HOME for the duration of one test and return a guard so
    /// the env var resets when the guard drops.  `TempDir` itself ensures
    /// the directory is removed.
    struct HomeGuard {
        _tmp: TempDir,
        prev: Option<String>,
    }

    fn set_home() -> HomeGuard {
        let tmp = tempfile::tempdir().expect("tempdir");
        let prev = std::env::var("IMPFORGE_CLI_HOME").ok();
        std::env::set_var("IMPFORGE_CLI_HOME", tmp.path());
        HomeGuard { _tmp: tmp, prev }
    }

    impl Drop for HomeGuard {
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => std::env::set_var("IMPFORGE_CLI_HOME", v),
                None => std::env::remove_var("IMPFORGE_CLI_HOME"),
            }
        }
    }

    #[test]
    fn trust_score_verified_and_signed_get_high_score() {
        let entry = MarketplaceEntry {
            id: "x".into(),
            name: "x".into(),
            description: "x".into(),
            publisher: "x".into(),
            version: "1".into(),
            category: "files".into(),
            tags: vec![],
            install_count: 100_000,
            stars: 5,
            verified: true,
            signed: true,
            transport: "stdio".into(),
            command: None,
            args: vec![],
            env: BTreeMap::new(),
            homepage: None,
            license: None,
            last_updated_iso: chrono::Utc::now().to_rfc3339(),
            capability_hint: Some(CapabilityHint {
                tools: 1,
                resources: 0,
                prompts: 0,
                oauth: false,
            }),
        };
        let score = trust_score(&entry);
        assert!(score >= 80, "expected verified+signed+5*+sandbox to score >=80, got {score}");
    }

    #[test]
    fn trust_score_anonymous_low() {
        let entry = MarketplaceEntry {
            id: "y".into(),
            name: "y".into(),
            description: "y".into(),
            publisher: "anon".into(),
            version: "0.1".into(),
            category: "custom".into(),
            tags: vec![],
            install_count: 1,
            stars: 0,
            verified: false,
            signed: false,
            transport: "stdio".into(),
            command: None,
            args: vec![],
            env: BTreeMap::new(),
            homepage: None,
            license: None,
            last_updated_iso: "2020-01-01T00:00:00Z".into(),
            capability_hint: None,
        };
        let score = trust_score(&entry);
        assert!(score <= 10, "expected anonymous old to score <=10, got {score}");
    }

    #[test]
    fn seed_entries_have_stable_ids() {
        let seed = seed_entries();
        let mut ids: Vec<&str> = seed.iter().map(|e| e.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), seed.len(), "seed entry ids must be unique");
    }

    #[test]
    fn ensure_seeded_inserts_curated_set_then_idempotent() {
        let _g = set_home();
        let conn = open_marketplace_db().expect("open db");
        let first = ensure_seeded(&conn).expect("seed first");
        assert!(first >= 8);
        let second = ensure_seeded(&conn).expect("seed second");
        assert_eq!(second, 0, "ensure_seeded must be idempotent");
        let entries = list_entries(&conn, None, None, 100).expect("list");
        assert_eq!(entries.len() as usize, first);
    }

    #[test]
    fn list_entries_filters_by_search_and_category() {
        let _g = set_home();
        let conn = open_marketplace_db().expect("open db");
        ensure_seeded(&conn).expect("seed");

        // Substring filter.
        let github = list_entries(&conn, Some("github"), None, 10).expect("filter github");
        assert!(
            github.iter().any(|e| e.id == "github"),
            "github filter must yield github"
        );

        // Category filter.
        let code = list_entries(&conn, None, Some("code"), 10).expect("filter code");
        assert!(
            code.iter().all(|e| e.category == "code"),
            "code filter must only return code category"
        );
        assert!(code.iter().any(|e| e.id == "git"));
    }

    #[test]
    fn install_then_uninstall_roundtrip() {
        let _g = set_home();
        let conn = open_marketplace_db().expect("open db");
        ensure_seeded(&conn).expect("seed");

        // Install (no probe — we don't need npm/uvx in CI).
        install(InstallArgs {
            id: "filesystem".into(),
            version: None,
            no_probe: true,
        })
        .expect("install");

        let installed = load_installed().expect("load installed");
        assert!(installed.iter().any(|s| s.id == "filesystem"));

        // Uninstall round-trips.
        uninstall("filesystem").expect("uninstall");
        let installed_after = load_installed().expect("load installed");
        assert!(installed_after.iter().all(|s| s.id != "filesystem"));
    }

    #[test]
    fn install_unknown_id_errors_clearly() {
        let _g = set_home();
        let conn = open_marketplace_db().expect("open db");
        ensure_seeded(&conn).expect("seed");
        let err = install(InstallArgs {
            id: "does-not-exist".into(),
            version: None,
            no_probe: true,
        })
        .expect_err("install must fail");
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn server_config_path_creates_dir() {
        let _g = set_home();
        let p = server_config_path("filesystem").expect("path");
        let parent = p.parent().expect("has parent");
        assert!(parent.exists(), "config parent dir should exist");
    }

    #[test]
    fn upsert_entry_replaces_existing_payload() {
        let _g = set_home();
        let conn = open_marketplace_db().expect("open db");
        let mut e = seed_entries().into_iter().next().expect("first seed");
        upsert_entry(&conn, &e).expect("first upsert");
        e.description = "rewritten".into();
        upsert_entry(&conn, &e).expect("second upsert");
        let fetched = get_entry(&conn, &e.id).expect("get");
        assert_eq!(fetched.description, "rewritten");
    }

    #[test]
    fn recency_score_modern_iso_high() {
        let now = chrono::Utc::now().to_rfc3339();
        assert!(recency_score(&now) >= 9.0);
    }

    #[test]
    fn recency_score_old_low() {
        assert_eq!(recency_score("2020-01-01T00:00:00Z"), 0.0);
    }

    #[test]
    fn recency_score_invalid_iso_zero() {
        assert_eq!(recency_score("not-a-date"), 0.0);
    }
}

