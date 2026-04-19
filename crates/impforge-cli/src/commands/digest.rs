// SPDX-License-Identifier: MIT
//! `impforge-cli digest` — Feature 3 (Global Digest, Tier 1).
//!
//! Universal auto-ingest for the CLI tier: RSS feeds + local folder
//! watching.  Both flow into the same FTS5 knowledge base used by
//! `impforge-cli ingest` (Feature 2 Tier 1).
//!
//! ## Subcommands
//!
//! ```text
//! impforge-cli digest add-feed <url>
//! impforge-cli digest add-folder <path> [--recursive]
//! impforge-cli digest watch          # foreground daemon — polls feeds + watches folders
//! impforge-cli digest run-once       # single ingest pass over every source
//! impforge-cli digest list-sources
//! impforge-cli digest history --limit 20
//! ```
//!
//! ## Sources file
//!
//! Sources persist as a single JSON file at
//! `$IMPFORGE_CLI_HOME/digest-sources.json` (defaults to
//! `~/.impforge-cli/digest-sources.json`).  No DB tables, no migrations
//! — keeps the CLI a single static binary.
//!
//! ## Privacy (REGEL 000-BRIDGE-NOT-PROCESS)
//!
//! Every byte of every ingested document stays on the user's disk.  The
//! ONLY outbound traffic is RSS HTTP fetches against URLs the user
//! explicitly added — those go user-machine → feed-provider directly,
//! never through `impforge.com`.
//!
//! ## Recall anti-patterns we explicitly reject
//!
//!   1. **Off by default** — every source is opt-in.  No background
//!      capture starts without an `add-*` command.
//!   2. **Single-key pause** — `Ctrl+C` interrupts `digest watch` cleanly.
//!   3. **No clipboard / no screenshots** — Tier 1 (CLI) does NOT
//!      monitor clipboard or screen content.  That's an opt-in feature
//!      of impforge-app (Tier 2).
//!   4. **Provenance every ingest** — every chunk inherits its source
//!      URL or file path through the `documents.path` column.
//!   5. **Zero telemetry** — nothing leaves the CLI process beyond the
//!      RSS fetches the user themselves configured.

use anyhow::{anyhow, bail, Context, Result};
use clap::{Args, Subcommand};
use impforge_emergence::Orchestrator;
use notify_debouncer_full::{
    new_debouncer,
    notify::{EventKind, RecommendedWatcher, RecursiveMode},
    DebounceEventResult, Debouncer, RecommendedCache,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::commands::ingest;
use crate::theme;

/// Default poll interval for RSS sources, when no per-source override is set.
pub const DEFAULT_POLL_SECS: u64 = 300; // 5 minutes — kind to feed servers

/// Hard ceiling on debouncer batches — see notify-debouncer-full README.
pub const DEBOUNCER_TIMEOUT: Duration = Duration::from_millis(750);

/// Hard cap on bytes the CLI digest will fetch from any single feed in
/// one pull.  Stops a misconfigured RSS endpoint from filling disk.
pub const MAX_FEED_BYTES: usize = 8 * 1024 * 1024;

/// User-Agent used for every RSS fetch.  Identifies us as a polite
/// reader so feed maintainers can see the load.
pub const FEED_USER_AGENT: &str = concat!(
    "ImpForge-CLI/",
    env!("CARGO_PKG_VERSION"),
    " (+https://impforge.com)"
);

// ─── Subcommand wiring ────────────────────────────────────────────────────

/// `impforge-cli digest …` command tree.
#[derive(Debug, Subcommand)]
pub enum DigestCmd {
    /// Add an RSS / Atom / JSON-Feed URL to the digest list.
    AddFeed(AddFeedArgs),
    /// Add a local folder for auto-ingest.
    AddFolder(AddFolderArgs),
    /// Run every source once + exit.
    RunOnce,
    /// Foreground daemon: polls RSS + watches folders until Ctrl+C.
    Watch(WatchArgs),
    /// Print every registered source to stdout.
    ListSources,
    /// Show recent ingest history.
    History(HistoryArgs),
    /// Remove a source by ID.
    Remove(RemoveArgs),
    /// Pause / resume the entire digest daemon.
    Pause,
    /// Resume after a pause.
    Resume,
}

#[derive(Debug, Args)]
pub struct AddFeedArgs {
    /// Feed URL.  Must be `http://` or `https://`.  Validated via the
    /// `url` crate.
    pub url: String,
    /// Override the per-feed poll interval (seconds).  Capped at 60 s
    /// minimum — any lower would be impolite to feed servers.
    #[arg(long, default_value_t = DEFAULT_POLL_SECS)]
    pub interval_secs: u64,
}

#[derive(Debug, Args)]
pub struct AddFolderArgs {
    /// Folder path.  Must be a directory the user can read.
    pub path: PathBuf,
    /// Watch sub-directories recursively.
    #[arg(long)]
    pub recursive: bool,
    /// Comma-separated extensions to treat as plaintext (passed straight
    /// to `impforge-cli ingest`).
    #[arg(long, value_delimiter = ',')]
    pub allow_ext: Vec<String>,
}

#[derive(Debug, Args)]
pub struct WatchArgs {
    /// Override the global RSS poll interval (seconds).  Per-source
    /// intervals still take precedence; this only affects sources
    /// without their own `interval_secs`.
    #[arg(long)]
    pub interval_secs: Option<u64>,
}

#[derive(Debug, Args)]
pub struct HistoryArgs {
    /// Number of recent ingest events to show.
    #[arg(long, default_value_t = 20)]
    pub limit: u64,
}

#[derive(Debug, Args)]
pub struct RemoveArgs {
    /// Source ID printed by `digest list-sources`.
    pub id: String,
}

// ─── Persisted state ──────────────────────────────────────────────────────

/// One configured digest source.  Persisted as JSON in `digest-sources.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DigestSource {
    /// HTTP(S) feed — RSS / Atom / JSON Feed.
    Feed {
        id: String,
        url: String,
        interval_secs: u64,
        /// Last `Last-Modified` / `ETag` we saw.  Used for conditional
        /// GET to avoid re-fetching unchanged feeds.
        #[serde(default)]
        last_modified: Option<String>,
        #[serde(default)]
        etag: Option<String>,
        #[serde(default)]
        last_polled_unix: u64,
    },
    /// Local folder — auto-ingest on file create / modify.
    Folder {
        id: String,
        path: PathBuf,
        recursive: bool,
        #[serde(default)]
        allow_ext: Vec<String>,
    },
}

impl DigestSource {
    pub fn id(&self) -> &str {
        match self {
            DigestSource::Feed { id, .. } => id,
            DigestSource::Folder { id, .. } => id,
        }
    }

    pub fn label(&self) -> String {
        match self {
            DigestSource::Feed { url, .. } => format!("feed: {url}"),
            DigestSource::Folder { path, recursive, .. } => format!(
                "folder: {} ({})",
                path.display(),
                if *recursive { "recursive" } else { "flat" }
            ),
        }
    }
}

/// Daemon-level pause flag.  Persisted alongside the sources file.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct DigestState {
    #[serde(default)]
    pub paused: bool,
    #[serde(default)]
    pub sources: Vec<DigestSource>,
}

/// One row of digest history — also persisted as JSON for simplicity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigestHistory {
    pub at_unix: u64,
    pub source_id: String,
    /// Stable shape so `digest history` can render anything.
    pub kind: String,
    /// e.g. "5 docs, 12 chunks" or the feed entry title.
    pub summary: String,
}

/// Default location of both `digest-sources.json` + `digest-history.jsonl`.
pub fn digest_home() -> Result<PathBuf> {
    let dir = if let Ok(custom) = std::env::var("IMPFORGE_CLI_HOME") {
        PathBuf::from(custom)
    } else {
        let home = dirs::home_dir().ok_or_else(|| anyhow!("no HOME directory"))?;
        home.join(".impforge-cli")
    };
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("creating impforge-cli home {}", dir.display()))?;
    Ok(dir)
}

fn sources_path() -> Result<PathBuf> {
    Ok(digest_home()?.join("digest-sources.json"))
}

fn history_path() -> Result<PathBuf> {
    Ok(digest_home()?.join("digest-history.jsonl"))
}

/// Load the persisted [`DigestState`].  Returns the default (empty,
/// not paused) when the file doesn't exist yet.
pub fn load_state() -> Result<DigestState> {
    let path = sources_path()?;
    if !path.exists() {
        return Ok(DigestState::default());
    }
    let bytes = std::fs::read(&path)
        .with_context(|| format!("reading {}", path.display()))?;
    let state: DigestState = serde_json::from_slice(&bytes)
        .with_context(|| format!("parsing {}", path.display()))?;
    Ok(state)
}

/// Atomically persist the [`DigestState`].  Writes to `<file>.tmp` +
/// `rename` so a crash mid-write can never corrupt the on-disk JSON.
pub fn save_state(state: &DigestState) -> Result<()> {
    let path = sources_path()?;
    let tmp = path.with_extension("json.tmp");
    let bytes =
        serde_json::to_vec_pretty(state).context("serialising digest state")?;
    std::fs::write(&tmp, &bytes)
        .with_context(|| format!("writing {}", tmp.display()))?;
    std::fs::rename(&tmp, &path)
        .with_context(|| format!("renaming to {}", path.display()))?;
    Ok(())
}

/// Append one history row.
pub fn append_history(row: &DigestHistory) -> Result<()> {
    let path = history_path()?;
    let mut line = serde_json::to_string(row).context("serialising history row")?;
    line.push('\n');
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("opening history {}", path.display()))?;
    use std::io::Write;
    f.write_all(line.as_bytes())
        .context("appending history row")?;
    Ok(())
}

/// Read the last `limit` history rows in reverse-chronological order.
pub fn tail_history(limit: usize) -> Result<Vec<DigestHistory>> {
    let path = history_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("reading history {}", path.display()))?;
    let mut out: Vec<DigestHistory> = raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    out.reverse();
    out.truncate(limit);
    Ok(out)
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Monotonic counter for source IDs.  Combined with current time to
/// produce identifiers that sort lexicographically.  Avoids pulling
/// `uuid` into the CLI binary (already heavy enough at <3 MB).
fn next_source_token() -> String {
    use std::sync::atomic::AtomicU64;
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_micros() as u64)
        .unwrap_or(0);
    format!("{t:016x}-{n:04x}")
}

// ─── Add / remove / list ──────────────────────────────────────────────────

fn add_feed(state: &mut DigestState, args: AddFeedArgs) -> Result<DigestSource> {
    let parsed = url::Url::parse(&args.url)
        .with_context(|| format!("invalid URL: {}", args.url))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        bail!("feed URL must be http(s): {}", args.url);
    }
    let interval_secs = args.interval_secs.max(60); // never poll faster than 1/min
    let src = DigestSource::Feed {
        id: format!("feed-{}", next_source_token()),
        url: parsed.into(),
        interval_secs,
        last_modified: None,
        etag: None,
        last_polled_unix: 0,
    };
    state.sources.push(src.clone());
    Ok(src)
}

fn add_folder(state: &mut DigestState, args: AddFolderArgs) -> Result<DigestSource> {
    let path = args.path.canonicalize().with_context(|| {
        format!("folder must exist + be readable: {}", args.path.display())
    })?;
    if !path.is_dir() {
        bail!("path is not a directory: {}", path.display());
    }
    let src = DigestSource::Folder {
        id: format!("folder-{}", next_source_token()),
        path,
        recursive: args.recursive,
        allow_ext: args.allow_ext,
    };
    state.sources.push(src.clone());
    Ok(src)
}

fn remove_source(state: &mut DigestState, id: &str) -> Result<()> {
    let before = state.sources.len();
    state.sources.retain(|s| s.id() != id);
    if state.sources.len() == before {
        bail!("no source with id {id}");
    }
    Ok(())
}

fn list_sources(state: &DigestState) {
    if state.sources.is_empty() {
        theme::print_warning("no digest sources registered");
        println!(
            "  add one with: impforge-cli digest add-feed <url> | digest add-folder <path>"
        );
        return;
    }
    theme::print_info(&format!(
        "{} digest source(s) — paused: {}",
        state.sources.len(),
        state.paused
    ));
    for src in &state.sources {
        println!("  {}  {}", src.id(), src.label());
    }
}

// ─── RSS fetching ─────────────────────────────────────────────────────────

/// Parse the bytes of a feed response into entries.  Pure function — no
/// network I/O.  Useful in tests because we feed it canned XML.
pub fn parse_feed(bytes: &[u8]) -> Result<Vec<feed_rs::model::Entry>> {
    let parser = feed_rs::parser::Builder::new().build();
    let parsed = parser
        .parse(std::io::Cursor::new(bytes))
        .map_err(|e| anyhow!("feed parse error: {e}"))?;
    Ok(parsed.entries)
}

/// One outcome of a single RSS pull.  Returned so the daemon can log
/// + the run-once command can report.
///
/// `not_modified` distinguishes a 304 response (server says nothing
/// new since `If-Modified-Since` / `If-None-Match`) from a successful
/// 200 with a possibly-empty entry list.
#[derive(Debug, Clone, Default)]
pub struct FeedPullOutcome {
    pub not_modified: bool,
    pub new_entries: usize,
    pub bytes: usize,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

/// Pull a single feed.  Honours `If-None-Match` / `If-Modified-Since`.
/// On a 304, returns immediately with `not_modified=true`.
pub async fn pull_feed(
    client: &reqwest::Client,
    url: &str,
    etag: Option<&str>,
    last_modified: Option<&str>,
) -> Result<FeedPullOutcome> {
    let mut req = client.get(url).header("User-Agent", FEED_USER_AGENT);
    if let Some(e) = etag {
        req = req.header("If-None-Match", e);
    }
    if let Some(lm) = last_modified {
        req = req.header("If-Modified-Since", lm);
    }
    let resp = req
        .send()
        .await
        .with_context(|| format!("fetching feed {url}"))?;
    if resp.status() == reqwest::StatusCode::NOT_MODIFIED {
        return Ok(FeedPullOutcome {
            not_modified: true,
            ..Default::default()
        });
    }
    if !resp.status().is_success() {
        bail!("feed {url}: HTTP {}", resp.status());
    }
    let etag_out = resp
        .headers()
        .get(reqwest::header::ETAG)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let last_mod_out = resp
        .headers()
        .get(reqwest::header::LAST_MODIFIED)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let bytes = resp
        .bytes()
        .await
        .with_context(|| format!("reading feed body {url}"))?;
    if bytes.len() > MAX_FEED_BYTES {
        bail!(
            "feed {url}: body {} bytes exceeds cap {} (likely misconfigured endpoint)",
            bytes.len(),
            MAX_FEED_BYTES
        );
    }
    let entries = parse_feed(&bytes)?;
    Ok(FeedPullOutcome {
        not_modified: false,
        new_entries: entries.len(),
        bytes: bytes.len(),
        etag: etag_out,
        last_modified: last_mod_out,
    })
}

/// Convert one parsed feed entry into plain text + a stable title +
/// canonical URL (for provenance).  Drops HTML markup using a tiny
/// scrubber — feed-rs gives us `Content::body` raw.
fn entry_to_text(entry: &feed_rs::model::Entry) -> (String, String) {
    let title = entry
        .title
        .as_ref()
        .map(|t| t.content.clone())
        .unwrap_or_else(|| "untitled".to_string());
    let body = entry
        .summary
        .as_ref()
        .map(|s| s.content.as_str())
        .or(entry
            .content
            .as_ref()
            .and_then(|c| c.body.as_deref()))
        .unwrap_or("")
        .to_string();
    (title, strip_html(&body))
}

/// Tiny tag stripper — feed bodies often contain inline HTML.  We
/// don't pull a full HTML parser into the CLI binary just for this;
/// the existing trigram-aware FTS5 tokeniser handles the residual
/// markup gracefully.  Replaces `<p>` etc. with whitespace, which is
/// what we want.
pub fn strip_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if in_tag => {}
            _ => out.push(c),
        }
    }
    // Collapse runs of whitespace to single spaces.
    let mut compact = String::with_capacity(out.len());
    let mut prev_ws = false;
    for c in out.chars() {
        if c.is_whitespace() {
            if !prev_ws {
                compact.push(' ');
            }
            prev_ws = true;
        } else {
            compact.push(c);
            prev_ws = false;
        }
    }
    compact.trim().to_string()
}

/// Persist one feed entry into the FTS5 knowledge base via the same
/// transactional ingest that powers `impforge-cli ingest`.  We write
/// the entry's title + body into a temporary `.txt` file under
/// `~/.impforge-cli/digest-cache/` so the FTS5 row carries provenance
/// (`documents.path`).
fn persist_feed_entry(
    conn: &mut rusqlite::Connection,
    feed_url: &str,
    entry: &feed_rs::model::Entry,
) -> Result<usize> {
    let (title, body) = entry_to_text(entry);
    if body.trim().is_empty() {
        return Ok(0);
    }
    let cache_dir = digest_home()?.join("digest-cache");
    std::fs::create_dir_all(&cache_dir)
        .with_context(|| format!("creating cache {}", cache_dir.display()))?;
    // Stable filename per (feed, entry id) so re-fetching dedups via hash.
    let slug = entry
        .id
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .take(80)
        .collect::<String>();
    let cache_path = cache_dir.join(format!(
        "{}-{}.txt",
        sanitize_for_path(feed_url),
        if slug.is_empty() {
            "entry".to_string()
        } else {
            slug
        }
    ));
    let payload = format!("{title}\n\nSource: {feed_url}\n\n{body}\n");
    std::fs::write(&cache_path, payload)
        .with_context(|| format!("writing cached entry {}", cache_path.display()))?;

    let outcome = ingest::ingest_file(conn, &cache_path, &[], false)
        .context("ingesting feed entry")?;
    Ok(outcome
        .map(|o| if o.skipped_duplicate { 0 } else { o.chunk_count })
        .unwrap_or(0))
}

/// Return a slug safe to embed in a filename — collapses `/`, `:` etc.
fn sanitize_for_path(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .take(60)
        .collect()
}

// ─── Folder ingest pass ───────────────────────────────────────────────────

/// Scan a folder once + ingest every recognised file.  Re-uses the
/// `impforge-cli ingest` machinery so the dedup hash is shared across
/// `ingest` and `digest`.
pub fn ingest_folder_once(
    conn: &mut rusqlite::Connection,
    src: &DigestSource,
) -> Result<(usize, usize)> {
    let DigestSource::Folder {
        path,
        recursive,
        allow_ext,
        ..
    } = src
    else {
        bail!("expected Folder source, got {:?}", src);
    };

    use walkdir::WalkDir;
    let walker = if *recursive {
        WalkDir::new(path).into_iter()
    } else {
        WalkDir::new(path).max_depth(1).into_iter()
    };

    let mut files = 0usize;
    let mut chunks = 0usize;
    for entry in walker.filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        match ingest::ingest_file(conn, entry.path(), allow_ext, false) {
            Ok(Some(out)) => {
                files += 1;
                chunks += out.chunk_count;
            }
            Ok(None) => {}
            Err(e) => {
                tracing::warn!("digest folder skip {}: {}", entry.path().display(), e);
            }
        }
    }
    Ok((files, chunks))
}

/// Process a single set of debounced filesystem events: ingest the
/// modified files so the in-progress watch loop catches up.
pub fn handle_fs_events(
    conn: &mut rusqlite::Connection,
    src: &DigestSource,
    events: &DebounceEventResult,
) -> Result<usize> {
    let DigestSource::Folder { allow_ext, .. } = src else {
        bail!("handle_fs_events called on non-folder source");
    };
    let Ok(events) = events else {
        return Ok(0);
    };
    let mut paths = BTreeSet::new();
    for ev in events {
        match ev.kind {
            EventKind::Create(_) | EventKind::Modify(_) => {
                for p in &ev.paths {
                    if p.is_file() {
                        paths.insert(p.clone());
                    }
                }
            }
            _ => {}
        }
    }
    let mut chunks = 0usize;
    for p in paths {
        match ingest::ingest_file(conn, &p, allow_ext, false) {
            Ok(Some(out)) => chunks += out.chunk_count,
            Ok(None) => {}
            Err(e) => tracing::warn!("digest live ingest skip {}: {}", p.display(), e),
        }
    }
    Ok(chunks)
}

// ─── run-once + watch daemon ──────────────────────────────────────────────

/// Run every source once.  Synchronous-ish (uses tokio's runtime for
/// the async RSS fetches).
pub fn run_once_blocking(state: &mut DigestState) -> Result<()> {
    let db_path = ingest::knowledge_db_path()?;
    let mut conn = ingest::open_db(&db_path)?;

    let runtime = tokio::runtime::Runtime::new()
        .context("starting tokio runtime for RSS pull")?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .context("building reqwest client")?;

    let mut updated = false;
    for src in state.sources.clone().iter() {
        match src {
            DigestSource::Feed {
                id,
                url,
                etag,
                last_modified,
                ..
            } => {
                let outcome = runtime.block_on(pull_feed(
                    &client,
                    url,
                    etag.as_deref(),
                    last_modified.as_deref(),
                ));
                match outcome {
                    Ok(o) => {
                        if o.not_modified {
                            theme::print_info(&format!("304 (unchanged): {url}"));
                        } else {
                            theme::print_success(&format!(
                                "feed {url} → {} bytes, {} entries",
                                o.bytes, o.new_entries
                            ));
                            // Re-fetch entries to persist; pull_feed
                            // returned only metadata.  We re-issue the
                            // GET to keep pull_feed cheap to test —
                            // pure RSS metadata path.  In a future
                            // refinement we can return the parsed
                            // entries directly and skip this round-trip.
                            if let Ok(resp) = runtime.block_on(
                                client
                                    .get(url)
                                    .header("User-Agent", FEED_USER_AGENT)
                                    .send(),
                            ) {
                                if let Ok(body) = runtime.block_on(resp.bytes()) {
                                    if let Ok(entries) = parse_feed(&body) {
                                        let mut total_chunks = 0usize;
                                        let mut entries_persisted = 0usize;
                                        for entry in &entries {
                                            match persist_feed_entry(&mut conn, url, entry)
                                            {
                                                Ok(c) if c > 0 => {
                                                    entries_persisted += 1;
                                                    total_chunks += c;
                                                }
                                                Ok(_) => {}
                                                Err(e) => tracing::warn!(
                                                    "digest persist entry skip: {e}"
                                                ),
                                            }
                                        }
                                        let _ = append_history(&DigestHistory {
                                            at_unix: now_unix(),
                                            source_id: id.clone(),
                                            kind: "feed".into(),
                                            summary: format!(
                                                "{entries_persisted} new entries · {total_chunks} chunks"
                                            ),
                                        });
                                    }
                                }
                            }
                        }
                        // Persist updated etag / last-modified.
                        for s in state.sources.iter_mut() {
                            if let DigestSource::Feed {
                                id: sid,
                                etag: e_slot,
                                last_modified: lm_slot,
                                last_polled_unix,
                                ..
                            } = s
                            {
                                if sid == id {
                                    *e_slot = o.etag.clone();
                                    *lm_slot = o.last_modified.clone();
                                    *last_polled_unix = now_unix();
                                    updated = true;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        theme::print_error(&format!("feed {url}: {e}"));
                        let _ = append_history(&DigestHistory {
                            at_unix: now_unix(),
                            source_id: id.clone(),
                            kind: "feed-error".into(),
                            summary: e.to_string(),
                        });
                    }
                }
            }
            DigestSource::Folder { id, .. } => {
                match ingest_folder_once(&mut conn, src) {
                    Ok((files, chunks)) => {
                        theme::print_success(&format!(
                            "folder {} → {files} files, {chunks} chunks",
                            src.label()
                        ));
                        let _ = append_history(&DigestHistory {
                            at_unix: now_unix(),
                            source_id: id.clone(),
                            kind: "folder".into(),
                            summary: format!("{files} files · {chunks} chunks"),
                        });
                    }
                    Err(e) => {
                        theme::print_error(&format!("folder {}: {e}", src.label()));
                        let _ = append_history(&DigestHistory {
                            at_unix: now_unix(),
                            source_id: id.clone(),
                            kind: "folder-error".into(),
                            summary: e.to_string(),
                        });
                    }
                }
            }
        }
    }
    if updated {
        save_state(state)?;
    }
    Ok(())
}

/// Watcher handle the daemon keeps alive for the lifetime of the run.
type WatcherType = Debouncer<RecommendedWatcher, RecommendedCache>;

/// One owned `(source_id, debouncer)` pair — kept alive on the
/// daemon's stack so the OS-level watchers don't get dropped.
type FolderWatchHandle = (String, WatcherType);

/// `(source_id, debounced events)` flowing into the daemon's event
/// loop.  Source ID lets the dispatcher route events to their source.
type FolderWatchEvent = (String, DebounceEventResult);

/// Receiver end of the channel the watchers push events into.
type FolderWatchRx = std::sync::mpsc::Receiver<FolderWatchEvent>;

/// Output of `install_folder_watchers`: the kept-alive watcher
/// handles + the receiver the daemon polls.
type FolderWatcherSetup = (Vec<FolderWatchHandle>, FolderWatchRx);

/// Bring up file-watcher debouncers for every Folder source.  We
/// register each source's path under its `watch_id` so the
/// `handle_fs_events` dispatcher can resolve event → source.
fn install_folder_watchers(state: &DigestState) -> Result<FolderWatcherSetup> {
    let (tx, rx) = channel::<FolderWatchEvent>();
    let mut watchers = Vec::new();
    for src in &state.sources {
        if let DigestSource::Folder { id, path, recursive, .. } = src {
            let id_owned = id.clone();
            let tx_clone = tx.clone();
            let mut debouncer = new_debouncer(
                DEBOUNCER_TIMEOUT,
                None,
                move |events: DebounceEventResult| {
                    let _ = tx_clone.send((id_owned.clone(), events));
                },
            )
            .with_context(|| format!("installing watcher on {}", path.display()))?;
            let mode = if *recursive {
                RecursiveMode::Recursive
            } else {
                RecursiveMode::NonRecursive
            };
            debouncer
                .watch(path, mode)
                .with_context(|| format!("watching {}", path.display()))?;
            watchers.push((id.clone(), debouncer));
        }
    }
    Ok((watchers, rx))
}

/// Foreground daemon — polls feeds + watches folders until Ctrl+C.
pub fn watch_blocking(
    args: WatchArgs,
    state: &mut DigestState,
) -> Result<()> {
    if state.paused {
        bail!(
            "digest is paused — run `impforge-cli digest resume` first \
             or remove the pause flag from {}",
            sources_path()?.display()
        );
    }
    if state.sources.is_empty() {
        bail!(
            "no sources to watch — add one with `impforge-cli digest add-feed` \
             or `add-folder`"
        );
    }
    let interval_override = args.interval_secs;

    theme::print_info("digest watch — Ctrl+C to stop");
    println!("  sources file: {}", sources_path()?.display());
    println!("  history file: {}", history_path()?.display());

    let db_path = ingest::knowledge_db_path()?;
    let mut conn = ingest::open_db(&db_path)?;

    let runtime = tokio::runtime::Runtime::new()
        .context("starting tokio runtime")?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .context("building reqwest client")?;

    let (watchers, rx_fs) = install_folder_watchers(state)?;
    let _watchers_kept_alive = watchers;

    // Ctrl+C handler — sets a flag the loop checks.  We install
    // tokio::signal::ctrl_c on the existing runtime and let it set
    // the atomic; portable across Linux/macOS/Windows without pulling
    // libc or the `ctrlc` crate.
    let stop_flag = Arc::new(AtomicBool::new(false));
    {
        let f = stop_flag.clone();
        let handle = runtime.handle().clone();
        std::thread::spawn(move || {
            handle.block_on(async move {
                if tokio::signal::ctrl_c().await.is_ok() {
                    f.store(true, Ordering::SeqCst);
                }
            });
        });
    }

    loop {
        if stop_flag.load(Ordering::SeqCst) {
            theme::print_info("digest watch stopped (Ctrl+C)");
            save_state(state)?;
            return Ok(());
        }

        // Drain any filesystem events (non-blocking up to 250ms).
        match rx_fs.recv_timeout(Duration::from_millis(250)) {
            Ok((source_id, events)) => {
                if let Some(src) =
                    state.sources.iter().find(|s| s.id() == source_id)
                {
                    if let Ok(chunks) = handle_fs_events(&mut conn, src, &events) {
                        if chunks > 0 {
                            theme::print_success(&format!(
                                "live ingest from {} → {} chunks",
                                src.label(),
                                chunks
                            ));
                            let _ = append_history(&DigestHistory {
                                at_unix: now_unix(),
                                source_id: source_id.clone(),
                                kind: "folder-live".into(),
                                summary: format!("{chunks} chunks"),
                            });
                        }
                    }
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                bail!("digest watcher channel disconnected unexpectedly");
            }
        }

        // Poll due feeds.
        let now = now_unix();
        let mut updated_any = false;
        for src in state.sources.clone().iter() {
            if let DigestSource::Feed {
                id,
                url,
                interval_secs,
                etag,
                last_modified,
                last_polled_unix,
            } = src
            {
                let interval = interval_override.unwrap_or(*interval_secs).max(60);
                if now.saturating_sub(*last_polled_unix) < interval {
                    continue;
                }
                let outcome = runtime.block_on(pull_feed(
                    &client,
                    url,
                    etag.as_deref(),
                    last_modified.as_deref(),
                ));
                match outcome {
                    Ok(o) => {
                        if !o.not_modified {
                            // Re-fetch + persist entries (see run_once_blocking).
                            if let Ok(resp) = runtime.block_on(
                                client
                                    .get(url)
                                    .header("User-Agent", FEED_USER_AGENT)
                                    .send(),
                            ) {
                                if let Ok(body) = runtime.block_on(resp.bytes()) {
                                    if let Ok(entries) = parse_feed(&body) {
                                        let mut total_chunks = 0usize;
                                        let mut new_entries = 0usize;
                                        for entry in &entries {
                                            match persist_feed_entry(&mut conn, url, entry)
                                            {
                                                Ok(c) if c > 0 => {
                                                    new_entries += 1;
                                                    total_chunks += c;
                                                }
                                                Ok(_) => {}
                                                Err(e) => tracing::warn!(
                                                    "digest persist skip: {e}"
                                                ),
                                            }
                                        }
                                        if new_entries > 0 {
                                            theme::print_success(&format!(
                                                "feed {url} → {new_entries} new · {total_chunks} chunks"
                                            ));
                                            let _ = append_history(&DigestHistory {
                                                at_unix: now,
                                                source_id: id.clone(),
                                                kind: "feed".into(),
                                                summary: format!(
                                                    "{new_entries} new entries · {total_chunks} chunks"
                                                ),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                        for s in state.sources.iter_mut() {
                            if let DigestSource::Feed {
                                id: sid,
                                etag: e_slot,
                                last_modified: lm_slot,
                                last_polled_unix: lp,
                                ..
                            } = s
                            {
                                if sid == id {
                                    *e_slot = o.etag.clone();
                                    *lm_slot = o.last_modified.clone();
                                    *lp = now;
                                    updated_any = true;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("feed pull {url}: {e}");
                    }
                }
            }
        }
        if updated_any {
            save_state(state)?;
        }

        std::thread::sleep(Duration::from_millis(250));
    }
}

// ─── Pause / resume ───────────────────────────────────────────────────────

fn set_paused(state: &mut DigestState, paused: bool) -> Result<()> {
    state.paused = paused;
    save_state(state)?;
    Ok(())
}

// ─── History rendering ────────────────────────────────────────────────────

fn print_history(rows: &[DigestHistory]) {
    if rows.is_empty() {
        theme::print_warning("no digest history yet");
        println!("  hint: run `impforge-cli digest run-once` to bootstrap");
        return;
    }
    theme::print_info(&format!("recent digest history — {} rows", rows.len()));
    for row in rows {
        let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(row.at_unix as i64, 0)
            .map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| row.at_unix.to_string());
        println!(
            "  {dt}  [{kind}] {summary}  (source={source})",
            kind = row.kind,
            summary = row.summary,
            source = row.source_id
        );
    }
}

// ─── Entry point dispatched from `main.rs` ────────────────────────────────

/// Top-level dispatcher.  Loads / saves state per command; keeps
/// every code path aware of pause + sources file invariants.
pub fn run(cmd: DigestCmd, _orchestrator: &Arc<Orchestrator>) -> Result<()> {
    let mut state = load_state().context("loading digest state")?;
    match cmd {
        DigestCmd::AddFeed(args) => {
            let src = add_feed(&mut state, args)?;
            save_state(&state)?;
            theme::print_success(&format!("added {} → {}", src.id(), src.label()));
        }
        DigestCmd::AddFolder(args) => {
            let src = add_folder(&mut state, args)?;
            save_state(&state)?;
            theme::print_success(&format!("added {} → {}", src.id(), src.label()));
        }
        DigestCmd::Remove(args) => {
            remove_source(&mut state, &args.id)?;
            save_state(&state)?;
            theme::print_success(&format!("removed {}", args.id));
        }
        DigestCmd::ListSources => list_sources(&state),
        DigestCmd::RunOnce => {
            if state.paused {
                bail!("digest is paused — run `impforge-cli digest resume` first");
            }
            run_once_blocking(&mut state)?;
        }
        DigestCmd::Watch(args) => watch_blocking(args, &mut state)?,
        DigestCmd::History(args) => {
            let rows = tail_history(args.limit as usize)?;
            print_history(&rows);
        }
        DigestCmd::Pause => {
            set_paused(&mut state, true)?;
            theme::print_success("digest paused");
        }
        DigestCmd::Resume => {
            set_paused(&mut state, false)?;
            theme::print_success("digest resumed");
        }
    }
    Ok(())
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Process-wide lock so tests that mutate the `IMPFORGE_CLI_HOME`
    /// env var don't race when cargo runs them in parallel threads.
    /// One serialised guard per test → real isolation.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Guard returned by `isolated_home`: holds both the env-mutex
    /// guard + the tempdir so they drop together at end-of-test.
    /// Both fields are owned-only — never read directly — but their
    /// lifetimes are exactly what the test needs.
    struct HomeGuard {
        _lock: std::sync::MutexGuard<'static, ()>,
        _dir: tempfile::TempDir,
    }

    /// Returns a HomeGuard — keep alive for the duration of the test.
    fn isolated_home() -> HomeGuard {
        // `lock()` returns Err only on poison; tests should still get
        // a chance to run with a fresh state, so map poison → drop.
        let guard = match ENV_LOCK.lock() {
            Ok(g) => g,
            Err(p) => {
                ENV_LOCK.clear_poison();
                p.into_inner()
            }
        };
        let dir = tempfile::tempdir().expect("tempdir");
        std::env::set_var("IMPFORGE_CLI_HOME", dir.path());
        HomeGuard {
            _lock: guard,
            _dir: dir,
        }
    }

    #[test]
    fn state_default_is_empty_and_unpaused() {
        let _h = isolated_home();
        let state = load_state().expect("load");
        assert!(state.sources.is_empty());
        assert!(!state.paused);
    }

    #[test]
    fn add_feed_validates_url() {
        let _h = isolated_home();
        let mut state = load_state().expect("load");
        let bad = add_feed(
            &mut state,
            AddFeedArgs {
                url: "not a url".to_string(),
                interval_secs: 300,
            },
        );
        assert!(bad.is_err());
    }

    #[test]
    fn add_feed_rejects_non_http_scheme() {
        let _h = isolated_home();
        let mut state = load_state().expect("load");
        let bad = add_feed(
            &mut state,
            AddFeedArgs {
                url: "ftp://example.com/feed".to_string(),
                interval_secs: 300,
            },
        );
        assert!(bad.is_err());
    }

    #[test]
    fn add_feed_clamps_interval_minimum() {
        let _h = isolated_home();
        let mut state = load_state().expect("load");
        let src = add_feed(
            &mut state,
            AddFeedArgs {
                url: "https://example.com/rss.xml".to_string(),
                interval_secs: 5, // too low
            },
        )
        .expect("add");
        if let DigestSource::Feed { interval_secs, .. } = src {
            assert_eq!(interval_secs, 60, "interval clamped to 60s minimum");
        } else {
            panic!("expected Feed");
        }
    }

    #[test]
    fn add_folder_requires_existing_dir() {
        let _h = isolated_home();
        let mut state = load_state().expect("load");
        let bad = add_folder(
            &mut state,
            AddFolderArgs {
                path: PathBuf::from("/nonexistent/totally/missing"),
                recursive: true,
                allow_ext: vec![],
            },
        );
        assert!(bad.is_err());
    }

    #[test]
    fn add_folder_persists_through_state_roundtrip() {
        let _h = isolated_home();
        let mut state = load_state().expect("load");
        let tmp = tempfile::tempdir().expect("tempdir for folder");
        let src = add_folder(
            &mut state,
            AddFolderArgs {
                path: tmp.path().to_path_buf(),
                recursive: true,
                allow_ext: vec!["log".to_string()],
            },
        )
        .expect("add");
        save_state(&state).expect("save");

        let loaded = load_state().expect("reload");
        assert_eq!(loaded.sources.len(), 1);
        assert_eq!(loaded.sources[0].id(), src.id());
    }

    #[test]
    fn remove_source_errors_on_missing_id() {
        let _h = isolated_home();
        let mut state = load_state().expect("load");
        let bad = remove_source(&mut state, "feed-nope");
        assert!(bad.is_err());
    }

    #[test]
    fn pause_resume_persists() {
        let _h = isolated_home();
        let mut state = load_state().expect("load");
        set_paused(&mut state, true).expect("pause");
        assert!(load_state().expect("reload").paused);
        set_paused(&mut state, false).expect("resume");
        assert!(!load_state().expect("reload").paused);
    }

    #[test]
    fn append_then_tail_history() {
        let _h = isolated_home();
        for i in 0..5 {
            append_history(&DigestHistory {
                at_unix: 1_000 + i as u64,
                source_id: format!("src-{i}"),
                kind: "test".into(),
                summary: format!("entry {i}"),
            })
            .expect("append");
        }
        let tailed = tail_history(3).expect("tail");
        assert_eq!(tailed.len(), 3);
        // Reverse-chronological → first row is the newest (at_unix=1004).
        assert_eq!(tailed[0].at_unix, 1004);
    }

    #[test]
    fn parse_feed_handles_atom() {
        let atom = br#"<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Test</title>
  <id>urn:test</id>
  <updated>2026-04-19T00:00:00Z</updated>
  <entry>
    <id>urn:e1</id>
    <title>Hello world</title>
    <updated>2026-04-19T00:00:00Z</updated>
    <summary>Just a test</summary>
  </entry>
</feed>"#;
        let entries = parse_feed(atom).expect("atom");
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].title.as_ref().expect("title").content,
            "Hello world"
        );
    }

    #[test]
    fn parse_feed_handles_rss2() {
        let rss = br#"<?xml version="1.0"?>
<rss version="2.0">
  <channel>
    <title>Test</title>
    <link>https://example.com</link>
    <description>x</description>
    <item>
      <title>RSS Item</title>
      <description>Body</description>
      <link>https://example.com/1</link>
    </item>
  </channel>
</rss>"#;
        let entries = parse_feed(rss).expect("rss");
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn strip_html_collapses_tags_and_whitespace() {
        let html = "<p>Hello   <b>world</b></p>\n<p>second</p>";
        let stripped = strip_html(html);
        assert_eq!(stripped, "Hello world second");
    }

    #[test]
    fn strip_html_handles_empty_input() {
        assert_eq!(strip_html(""), "");
    }

    #[test]
    fn entry_to_text_falls_back_to_untitled() {
        // Round-trip via parser to avoid coupling to feed-rs's private
        // text constructors — minimal Atom feed without `<title>` on
        // the entry.
        let atom = br#"<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>F</title>
  <id>urn:f</id>
  <updated>2026-04-19T00:00:00Z</updated>
  <entry>
    <id>urn:e</id>
    <updated>2026-04-19T00:00:00Z</updated>
    <summary>&lt;p&gt;hello&lt;/p&gt;</summary>
  </entry>
</feed>"#;
        let entries = parse_feed(atom).expect("parse");
        let entry = entries.into_iter().next().expect("one entry");
        let (title, body) = entry_to_text(&entry);
        assert_eq!(title, "untitled");
        assert_eq!(body, "hello");
    }

    #[test]
    fn folder_watcher_detects_new_file() {
        // Behavioural test: install a watcher on a tempdir, drop a
        // file in, assert the channel sees it.
        let _h = isolated_home();
        let tmp = tempfile::tempdir().expect("tempdir for watcher");
        let mut state = load_state().expect("load");
        let _src = add_folder(
            &mut state,
            AddFolderArgs {
                path: tmp.path().to_path_buf(),
                recursive: false,
                allow_ext: vec!["txt".into()],
            },
        )
        .expect("add folder");
        save_state(&state).expect("save");

        let (_watchers, rx) = install_folder_watchers(&state).expect("install");
        // Give the OS time to register the watch.
        std::thread::sleep(Duration::from_millis(150));

        let target = tmp.path().join("hello.txt");
        std::fs::write(&target, b"behavioural").expect("write");

        // Wait up to debouncer + grace.
        let received = rx.recv_timeout(DEBOUNCER_TIMEOUT + Duration::from_secs(2));
        assert!(
            received.is_ok(),
            "watcher should have produced an event for the new file"
        );
    }

    #[test]
    fn ingest_folder_once_picks_up_files() {
        // Behavioural test: drop a markdown file into a folder source
        // and confirm `ingest_folder_once` returns chunk_count > 0.
        let _h = isolated_home();
        let tmp = tempfile::tempdir().expect("tempdir for ingest");
        std::fs::write(
            tmp.path().join("note.md"),
            "# Hello\n\nBody text for chunking.\n",
        )
        .expect("write md");

        let src = DigestSource::Folder {
            id: "folder-test".into(),
            path: tmp.path().to_path_buf(),
            recursive: false,
            allow_ext: vec![],
        };

        let db_path = ingest::knowledge_db_path().expect("db path");
        let mut conn = ingest::open_db(&db_path).expect("open db");
        let (files, chunks) =
            ingest_folder_once(&mut conn, &src).expect("ingest");
        assert!(files >= 1, "expected ≥1 file, got {files}");
        assert!(chunks >= 1, "expected ≥1 chunk, got {chunks}");
    }

    #[test]
    fn sanitize_for_path_strips_unsafe_chars() {
        let s = sanitize_for_path("https://example.com/feed?x=1");
        assert!(s.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_'));
    }
}
