// SPDX-License-Identifier: MIT
//! `impforge-cli ingest` — Feature 2 (Document Upload + RAG, Tier 1/3).
//!
//! Indexes plain text + Markdown + PDF documents into a local SQLite FTS5
//! database at `~/.impforge-cli/knowledge.db`.  Tier 1 is intentionally
//! lean: line-based chunking, single porter+unicode61 FTS5 virtual table,
//! no embeddings.  The App tier upgrades to dual-table (porter + trigram)
//! + Reciprocal Rank Fusion.  The Pro tier adds embeddings + Wikidata KG.
//!
//! ## Hard limits (MIT FREE tier)
//!
//! * 10 documents — beyond, we surface an upgrade hook to ImpForge Pro.
//! * Single FTS5 ranking (BM25) — no RRF, no vector search.
//!
//! ## Privacy
//!
//! Every byte stays on the user's disk.  No content ever uploaded to
//! `impforge.com` or any other endpoint.  Per REGEL 000-BRIDGE-NOT-PROCESS.

use anyhow::{anyhow, bail, Context, Result};
use clap::Args;
use impforge_emergence::Orchestrator;
use rusqlite::{params, Connection, OpenFlags};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use walkdir::WalkDir;

use crate::theme;

/// Hard limit on the number of distinct documents the MIT tier may index.
/// This is the upgrade-hook trigger.
pub const MIT_MAX_DOCUMENTS: usize = 10;

/// Default chunk size in lines.  Picked to be small enough that a single
/// FTS5 hit highlights a single thought, large enough to keep DB row
/// count reasonable on small disks.
const CHUNK_LINES: usize = 12;

/// CLI flags for `impforge-cli ingest`.
#[derive(Debug, Args)]
pub struct IngestArgs {
    /// File or directory to ingest.  Supported formats: `.pdf`, `.md`,
    /// `.markdown`, `.txt`, plus any extension in `--allow-ext`.
    pub path: PathBuf,
    /// When `path` is a directory, descend into subdirectories.
    #[arg(long)]
    pub recursive: bool,
    /// Allow extra extensions (case-insensitive, comma-separated, no dot).
    /// Treated as plain text.  Example: `--allow-ext=log,csv,toml`.
    #[arg(long, value_delimiter = ',')]
    pub allow_ext: Vec<String>,
    /// Re-ingest even if the file's content hash already exists.
    #[arg(long)]
    pub force: bool,
}

/// Return the on-disk path of the MIT FTS5 knowledge database.
///
/// Honours `IMPFORGE_CLI_HOME` for tests and BYO-vault deployments;
/// falls back to `~/.impforge-cli/knowledge.db`.
pub fn knowledge_db_path() -> Result<PathBuf> {
    let dir = if let Ok(custom) = std::env::var("IMPFORGE_CLI_HOME") {
        PathBuf::from(custom)
    } else {
        let home = dirs::home_dir().ok_or_else(|| anyhow!("could not resolve home directory"))?;
        home.join(".impforge-cli")
    };
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("creating knowledge dir {}", dir.display()))?;
    Ok(dir.join("knowledge.db"))
}

/// Recognised file formats at Tier 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliFormat {
    Pdf,
    Markdown,
    Plaintext,
}

impl CliFormat {
    /// Stable string used in the `documents.format` column.
    pub fn as_str(self) -> &'static str {
        match self {
            CliFormat::Pdf => "pdf",
            CliFormat::Markdown => "markdown",
            CliFormat::Plaintext => "plaintext",
        }
    }

    /// Detect by extension.  Returns `None` for unsupported types so the
    /// caller can decide whether to skip or treat as plaintext via
    /// `--allow-ext`.
    pub fn detect(path: &Path, allow_ext: &[String]) -> Option<Self> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())?;
        match ext.as_str() {
            "pdf" => Some(CliFormat::Pdf),
            "md" | "markdown" => Some(CliFormat::Markdown),
            "txt" => Some(CliFormat::Plaintext),
            other if allow_ext.iter().any(|allowed| allowed.eq_ignore_ascii_case(other)) => {
                Some(CliFormat::Plaintext)
            }
            _ => None,
        }
    }
}

/// Open or create the FTS5 knowledge database.  Creates schema on first run.
pub fn open_db(path: &Path) -> Result<Connection> {
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
    )
    .with_context(|| format!("opening sqlite at {}", path.display()))?;

    // WAL mode = crash-safe, multi-reader.  Single writer per process is
    // fine for a CLI; this isn't a server.
    conn.pragma_update(None, "journal_mode", "WAL")
        .context("enabling WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")
        .context("setting sync")?;

    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS documents (
            id           INTEGER PRIMARY KEY,
            path         TEXT UNIQUE NOT NULL,
            format       TEXT NOT NULL,
            title        TEXT,
            ingested_at  INTEGER NOT NULL,
            hash         TEXT NOT NULL,
            size_bytes   INTEGER NOT NULL
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS chunks USING fts5(
            text,
            doc_id    UNINDEXED,
            line_start UNINDEXED,
            line_end   UNINDEXED,
            tokenize='porter unicode61'
        );

        CREATE INDEX IF NOT EXISTS idx_documents_hash ON documents(hash);
        "#,
    )
    .context("creating schema")?;

    Ok(conn)
}

/// Read + parse a single file into a plain-text string ready for chunking.
fn extract_text(path: &Path, format: CliFormat) -> Result<String> {
    match format {
        CliFormat::Plaintext => std::fs::read_to_string(path)
            .with_context(|| format!("reading text {}", path.display())),
        CliFormat::Markdown => {
            let raw = std::fs::read_to_string(path)
                .with_context(|| format!("reading markdown {}", path.display()))?;
            Ok(strip_markdown(&raw))
        }
        CliFormat::Pdf => extract_pdf_text(path),
    }
}

/// Strip Markdown to plain text using `pulldown-cmark`'s event stream.
/// Headings keep their text (no `#`), code blocks stay verbatim, links
/// keep the visible label only.
pub fn strip_markdown(src: &str) -> String {
    use pulldown_cmark::{Event, Parser, Tag, TagEnd};

    let mut out = String::with_capacity(src.len());
    let parser = Parser::new(src);

    for event in parser {
        match event {
            Event::Text(t) | Event::Code(t) | Event::Html(t) | Event::InlineHtml(t) => {
                out.push_str(&t);
            }
            Event::SoftBreak | Event::HardBreak => out.push('\n'),
            Event::End(TagEnd::Paragraph)
            | Event::End(TagEnd::Heading(_))
            | Event::End(TagEnd::Item)
            | Event::End(TagEnd::CodeBlock) => out.push('\n'),
            Event::Start(Tag::CodeBlock(_)) => out.push('\n'),
            _ => {}
        }
    }
    out
}

/// Extract text from every page of a PDF using `lopdf`.  Pages are
/// joined by single newlines so chunking still finds reasonable breaks.
pub fn extract_pdf_text(path: &Path) -> Result<String> {
    let doc =
        lopdf::Document::load(path).with_context(|| format!("loading pdf {}", path.display()))?;
    let mut out = String::new();
    for (page_num, _) in doc.get_pages() {
        match doc.extract_text(&[page_num]) {
            Ok(text) => {
                out.push_str(&text);
                if !text.ends_with('\n') {
                    out.push('\n');
                }
            }
            Err(e) => {
                tracing::warn!(
                    "skipping page {} of {}: {}",
                    page_num,
                    path.display(),
                    e
                );
            }
        }
    }
    Ok(out)
}

/// Split text into line-bounded chunks of `CHUNK_LINES` lines each.  Returns
/// `(text, line_start, line_end)` triples — line numbers are 1-based.
pub fn chunk_text(text: &str) -> Vec<(String, usize, usize)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut idx = 0;
    while idx < lines.len() {
        let end = (idx + CHUNK_LINES).min(lines.len());
        let chunk = lines[idx..end].join("\n");
        if !chunk.trim().is_empty() {
            out.push((chunk, idx + 1, end));
        }
        idx = end;
    }
    out
}

/// SHA-256 hash of file bytes — used for dedup + change detection.
pub fn file_hash(path: &Path) -> Result<String> {
    let bytes =
        std::fs::read(path).with_context(|| format!("hashing {}", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

/// Outcome of ingesting one file — exposed for tests + the wider callers.
#[derive(Debug, Clone)]
pub struct IngestOutcome {
    pub path: PathBuf,
    pub format: CliFormat,
    pub doc_id: i64,
    pub chunk_count: usize,
    pub bytes: u64,
    pub skipped_duplicate: bool,
}

/// Insert (or refresh) a single file into the FTS5 index.
///
/// Returns `Ok(None)` when the file format is not recognised (caller
/// should keep walking).  Otherwise returns the `IngestOutcome` for
/// reporting.  Enforces `MIT_MAX_DOCUMENTS` BEFORE writing — the upgrade
/// hook is a hard error so users see the limit instead of a silent skip.
pub fn ingest_file(
    conn: &mut Connection,
    path: &Path,
    allow_ext: &[String],
    force: bool,
) -> Result<Option<IngestOutcome>> {
    let Some(format) = CliFormat::detect(path, allow_ext) else {
        return Ok(None);
    };

    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalising {}", path.display()))?;
    let hash = file_hash(&canonical)?;
    let size = std::fs::metadata(&canonical)
        .with_context(|| format!("stat {}", canonical.display()))?
        .len();

    // Dedup check by hash unless `--force`.
    if !force {
        let existing: Option<i64> = conn
            .query_row(
                "SELECT id FROM documents WHERE hash = ?1",
                params![&hash],
                |row| row.get(0),
            )
            .ok();
        if let Some(doc_id) = existing {
            return Ok(Some(IngestOutcome {
                path: canonical,
                format,
                doc_id,
                chunk_count: 0,
                bytes: size,
                skipped_duplicate: true,
            }));
        }
    }

    // MIT hard limit.  Counted against unique paths so re-ingesting the
    // same file with `--force` doesn't trip the gate.
    let path_str = canonical.to_string_lossy().to_string();
    let already_exists: Option<i64> = conn
        .query_row(
            "SELECT id FROM documents WHERE path = ?1",
            params![&path_str],
            |row| row.get(0),
        )
        .ok();
    if already_exists.is_none() {
        let count: i64 =
            conn.query_row("SELECT COUNT(*) FROM documents", [], |row| row.get(0))?;
        if (count as usize) >= MIT_MAX_DOCUMENTS {
            bail!(
                "MIT FREE tier limit reached: {} documents indexed (cap = {}). \
                 Upgrade to ImpForge Pro for unlimited ingest, vector search, \
                 Wikidata KG, and re-ranking. https://impforge.com",
                count,
                MIT_MAX_DOCUMENTS
            );
        }
    }

    // Extract + chunk.
    let text = extract_text(&canonical, format)?;
    let chunks = chunk_text(&text);
    let title = canonical
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document")
        .to_string();

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    // Transactional insert — either both `documents` row + every chunk
    // land or neither does.
    let tx = conn.transaction().context("starting ingest tx")?;
    let doc_id: i64;
    if let Some(existing_id) = already_exists {
        // Re-ingest path — wipe old chunks first.
        tx.execute(
            "DELETE FROM chunks WHERE doc_id = ?1",
            params![existing_id],
        )
        .context("clearing stale chunks")?;
        tx.execute(
            "UPDATE documents SET format=?1, title=?2, ingested_at=?3, hash=?4, size_bytes=?5 \
             WHERE id=?6",
            params![format.as_str(), &title, now, &hash, size as i64, existing_id],
        )
        .context("updating document row")?;
        doc_id = existing_id;
    } else {
        tx.execute(
            "INSERT INTO documents(path, format, title, ingested_at, hash, size_bytes) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![&path_str, format.as_str(), &title, now, &hash, size as i64],
        )
        .context("inserting document row")?;
        doc_id = tx.last_insert_rowid();
    }

    let mut chunk_count = 0usize;
    {
        let mut stmt = tx
            .prepare(
                "INSERT INTO chunks(text, doc_id, line_start, line_end) \
                 VALUES (?1, ?2, ?3, ?4)",
            )
            .context("preparing chunk insert")?;
        for (chunk_text, line_start, line_end) in &chunks {
            stmt.execute(params![
                chunk_text,
                doc_id,
                *line_start as i64,
                *line_end as i64,
            ])
            .context("inserting chunk")?;
            chunk_count += 1;
        }
    }
    tx.commit().context("committing ingest tx")?;

    Ok(Some(IngestOutcome {
        path: canonical,
        format,
        doc_id,
        chunk_count,
        bytes: size,
        skipped_duplicate: false,
    }))
}

/// Walk `path` and ingest every recognised file.  Single files are passed
/// straight through; directories are walked respecting `--recursive`.
pub fn run(args: IngestArgs, _orchestrator: &Arc<Orchestrator>) -> Result<()> {
    let db_path = knowledge_db_path()?;
    let mut conn = open_db(&db_path)?;
    theme::print_info("Ingest — local FTS5 knowledge base");
    println!("  knowledge db: {}", db_path.display());

    let mut total_files = 0usize;
    let mut total_chunks = 0usize;
    let mut total_bytes = 0u64;
    let mut skipped = 0usize;

    if args.path.is_file() {
        match ingest_file(&mut conn, &args.path, &args.allow_ext, args.force)? {
            Some(out) => {
                report_outcome(&out);
                total_files += 1;
                total_chunks += out.chunk_count;
                total_bytes += out.bytes;
                if out.skipped_duplicate {
                    skipped += 1;
                }
            }
            None => {
                println!(
                    "  skip (unsupported extension): {}",
                    args.path.display()
                );
            }
        }
    } else if args.path.is_dir() {
        let walker = if args.recursive {
            WalkDir::new(&args.path).into_iter()
        } else {
            WalkDir::new(&args.path).max_depth(1).into_iter()
        };
        for entry in walker.filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }
            match ingest_file(&mut conn, entry.path(), &args.allow_ext, args.force) {
                Ok(Some(out)) => {
                    report_outcome(&out);
                    total_files += 1;
                    total_chunks += out.chunk_count;
                    total_bytes += out.bytes;
                    if out.skipped_duplicate {
                        skipped += 1;
                    }
                }
                Ok(None) => {} // unsupported, silent skip in dir-walk mode
                Err(e) => {
                    eprintln!("  error: {} — {}", entry.path().display(), e);
                    return Err(e);
                }
            }
        }
    } else {
        bail!(
            "path does not exist or is not a file/directory: {}",
            args.path.display()
        );
    }

    println!(
        "\n  done: {} files · {} chunks · {} bytes · {} duplicates skipped",
        total_files, total_chunks, total_bytes, skipped
    );
    Ok(())
}

fn report_outcome(out: &IngestOutcome) {
    if out.skipped_duplicate {
        println!(
            "  dup  {}  ({})  doc_id={}",
            out.path.display(),
            out.format.as_str(),
            out.doc_id
        );
    } else {
        println!(
            "  ok   {}  ({})  doc_id={} chunks={}",
            out.path.display(),
            out.format.as_str(),
            out.doc_id,
            out.chunk_count
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn isolated_home() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        std::env::set_var("IMPFORGE_CLI_HOME", dir.path());
        dir
    }

    #[test]
    fn detect_format_by_extension() {
        let allow: Vec<String> = vec![];
        assert_eq!(
            CliFormat::detect(Path::new("a.PDF"), &allow),
            Some(CliFormat::Pdf)
        );
        assert_eq!(
            CliFormat::detect(Path::new("notes.MD"), &allow),
            Some(CliFormat::Markdown)
        );
        assert_eq!(
            CliFormat::detect(Path::new("readme.markdown"), &allow),
            Some(CliFormat::Markdown)
        );
        assert_eq!(
            CliFormat::detect(Path::new("a.txt"), &allow),
            Some(CliFormat::Plaintext)
        );
        assert_eq!(CliFormat::detect(Path::new("a.bin"), &allow), None);
    }

    #[test]
    fn detect_allow_ext_extends_plaintext() {
        let allow = vec!["log".to_string(), "csv".to_string()];
        assert_eq!(
            CliFormat::detect(Path::new("a.log"), &allow),
            Some(CliFormat::Plaintext)
        );
        assert_eq!(
            CliFormat::detect(Path::new("DATA.CSV"), &allow),
            Some(CliFormat::Plaintext)
        );
    }

    #[test]
    fn strip_markdown_keeps_text_only() {
        let md = "# Title\n\nSome **bold** text and `code`.\n\n```rust\nfn foo() {}\n```\n";
        let plain = strip_markdown(md);
        assert!(plain.contains("Title"));
        assert!(plain.contains("Some"));
        assert!(plain.contains("bold"));
        assert!(plain.contains("code"));
        assert!(plain.contains("fn foo()"));
        // Markdown syntax must be gone.
        assert!(!plain.contains("**"));
        assert!(!plain.contains("```"));
    }

    #[test]
    fn chunk_respects_chunk_lines_setting() {
        let text = (1..=30)
            .map(|n| format!("line {}", n))
            .collect::<Vec<_>>()
            .join("\n");
        let chunks = chunk_text(&text);
        assert_eq!(chunks.len(), 3); // 30 / 12 = 2.5 → 3 chunks
        assert_eq!(chunks[0].1, 1);
        assert_eq!(chunks[0].2, CHUNK_LINES);
        assert_eq!(chunks[2].2, 30);
    }

    #[test]
    fn chunk_drops_empty_chunks() {
        let chunks = chunk_text("\n\n   \n\n");
        assert!(chunks.is_empty());
    }

    #[test]
    fn ingest_text_file_creates_doc_and_chunks() {
        let _home = isolated_home();
        let db = knowledge_db_path().expect("db path");
        let mut conn = open_db(&db).expect("open db");

        let tmp = tempfile::NamedTempFile::with_suffix(".txt").expect("tmp");
        writeln!(tmp.as_file(), "The quick brown fox").expect("write");
        writeln!(tmp.as_file(), "jumps over the lazy dog").expect("write");
        writeln!(tmp.as_file(), "in the impforge knowledge base.").expect("write");

        let outcome = ingest_file(&mut conn, tmp.path(), &[], false)
            .expect("ingest")
            .expect("recognised");
        assert!(!outcome.skipped_duplicate);
        assert_eq!(outcome.format, CliFormat::Plaintext);
        assert!(outcome.chunk_count >= 1);
        assert!(outcome.bytes > 0);

        let doc_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM documents", [], |r| r.get(0))
            .expect("count");
        assert_eq!(doc_count, 1);

        let chunk_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM chunks", [], |r| r.get(0))
            .expect("count chunks");
        assert!(chunk_count >= 1);
    }

    #[test]
    fn ingest_dedups_by_hash_without_force() {
        let _home = isolated_home();
        let db = knowledge_db_path().expect("db path");
        let mut conn = open_db(&db).expect("open db");

        let tmp = tempfile::NamedTempFile::with_suffix(".txt").expect("tmp");
        writeln!(tmp.as_file(), "stable content").expect("write");

        let first = ingest_file(&mut conn, tmp.path(), &[], false)
            .expect("first")
            .expect("recognised");
        assert!(!first.skipped_duplicate);

        let second = ingest_file(&mut conn, tmp.path(), &[], false)
            .expect("second")
            .expect("recognised");
        assert!(second.skipped_duplicate, "second ingest should be a dup");
        assert_eq!(first.doc_id, second.doc_id);

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM documents", [], |r| r.get(0))
            .expect("count");
        assert_eq!(count, 1);
    }

    #[test]
    fn mit_limit_blocks_eleventh_document() {
        let _home = isolated_home();
        let db = knowledge_db_path().expect("db path");
        let mut conn = open_db(&db).expect("open db");

        // Each iteration writes UNIQUE content so the dedup path doesn't
        // exempt them from the MIT_MAX_DOCUMENTS limit.
        for i in 0..MIT_MAX_DOCUMENTS {
            let tmp = tempfile::NamedTempFile::with_suffix(".txt").expect("tmp");
            writeln!(tmp.as_file(), "doc number {}", i).expect("write");
            ingest_file(&mut conn, tmp.path(), &[], false)
                .expect("under limit ok")
                .expect("recognised");
        }

        let overflow = tempfile::NamedTempFile::with_suffix(".txt").expect("tmp");
        writeln!(overflow.as_file(), "this should be blocked").expect("write");
        let err = ingest_file(&mut conn, overflow.path(), &[], false)
            .expect_err("11th must trip MIT limit");
        let msg = format!("{err}");
        assert!(msg.contains("MIT FREE tier limit"), "got: {msg}");
        assert!(msg.contains("https://impforge.com"));
    }

    #[test]
    fn ingest_markdown_strips_syntax() {
        let _home = isolated_home();
        let db = knowledge_db_path().expect("db path");
        let mut conn = open_db(&db).expect("open db");

        let tmp = tempfile::NamedTempFile::with_suffix(".md").expect("tmp");
        writeln!(
            tmp.as_file(),
            "# Heading\n\nSome **bold** word for searching."
        )
        .expect("write");

        let outcome = ingest_file(&mut conn, tmp.path(), &[], false)
            .expect("ok")
            .expect("recognised");
        assert_eq!(outcome.format, CliFormat::Markdown);

        // The stored chunk must NOT contain the literal `**`.
        let chunk: String = conn
            .query_row(
                "SELECT text FROM chunks WHERE doc_id = ?1 LIMIT 1",
                params![outcome.doc_id],
                |row| row.get(0),
            )
            .expect("chunk");
        assert!(chunk.contains("Heading"));
        assert!(chunk.contains("bold"));
        assert!(!chunk.contains("**"));
    }
}
