// SPDX-License-Identifier: MIT
//! `impforge-cli search` — Feature 2 (Document Upload + RAG, Tier 1/3).
//!
//! Runs a ranked FTS5 query over the local knowledge database created by
//! `impforge-cli ingest`.  Tier 1 ranking is BM25 (built into SQLite's
//! FTS5 module).  The App tier upgrades to Reciprocal Rank Fusion across
//! porter+trigram tables; Pro adds embeddings + reranker.
//!
//! ## Privacy
//!
//! All search runs against the user's local SQLite database — no query,
//! no result, no document content is ever transmitted.  Per
//! REGEL 000-BRIDGE-NOT-PROCESS.

use anyhow::{Context, Result};
use clap::Args;
use impforge_emergence::Orchestrator;
use rusqlite::{params, Connection};
use std::sync::Arc;

use crate::commands::ingest::{knowledge_db_path, open_db};
use crate::theme;

/// CLI flags for `impforge-cli search`.
#[derive(Debug, Args)]
pub struct SearchArgs {
    /// FTS5 query.  Use bare words for keyword search; quote multi-word
    /// phrases (`"climate change"`); use FTS5 prefix syntax with a `*`
    /// (`temp*`); combine with `AND`/`OR`/`NOT`.
    pub query: String,
    /// Maximum number of result rows to return (default 10).
    #[arg(long, default_value_t = 10)]
    pub limit: u32,
    /// Force BM25 ordering (the default).  Kept for explicit users.
    #[arg(long)]
    pub bm25: bool,
    /// Output as one JSON object per line (machine-friendly).
    #[arg(long)]
    pub json: bool,
}

/// One ranked search hit.
#[derive(Debug, Clone)]
pub struct CliSearchHit {
    pub doc_id: i64,
    pub doc_path: String,
    pub doc_title: String,
    pub snippet: String,
    pub line_start: i64,
    pub line_end: i64,
    pub score: f64,
}

/// Run a ranked FTS5 query against the open database.
///
/// `score` is the RAW `bm25(chunks)` value as returned by SQLite — note
/// that this is a *negative* number where lower (more negative) values
/// mean better matches in FTS5 ranking semantics.  We sort ascending so
/// the most relevant chunk appears first, then return scores unmodified
/// (callers can format them however they like).
pub fn run_query(conn: &Connection, query: &str, limit: u32) -> Result<Vec<CliSearchHit>> {
    let mut stmt = conn
        .prepare(
            "SELECT \
                chunks.doc_id, \
                documents.path, \
                COALESCE(documents.title, ''), \
                snippet(chunks, 0, '<<', '>>', '…', 12), \
                chunks.line_start, \
                chunks.line_end, \
                bm25(chunks) AS score \
             FROM chunks \
             JOIN documents ON documents.id = chunks.doc_id \
             WHERE chunks MATCH ?1 \
             ORDER BY score ASC \
             LIMIT ?2",
        )
        .context("preparing FTS5 query")?;

    let rows = stmt
        .query_map(params![query, limit as i64], |row| {
            Ok(CliSearchHit {
                doc_id: row.get(0)?,
                doc_path: row.get(1)?,
                doc_title: row.get(2)?,
                snippet: row.get(3)?,
                line_start: row.get(4)?,
                line_end: row.get(5)?,
                score: row.get(6)?,
            })
        })
        .context("executing FTS5 query")?;

    let mut hits = Vec::new();
    for row in rows {
        hits.push(row.context("row decode")?);
    }
    Ok(hits)
}

/// Top-level command runner.
pub fn run(args: SearchArgs, _orchestrator: &Arc<Orchestrator>) -> Result<()> {
    let db_path = knowledge_db_path()?;
    let conn = open_db(&db_path)?;
    if !args.json {
        theme::print_info("Search — local FTS5 knowledge base (BM25)");
        println!("  knowledge db: {}", db_path.display());
        println!("  query: {}", args.query);
    }

    let hits = run_query(&conn, &args.query, args.limit)?;

    if hits.is_empty() {
        if args.json {
            println!("[]");
        } else {
            println!("\n  no matches.");
        }
        return Ok(());
    }

    if args.json {
        let mut buf = String::from("[\n");
        for (i, hit) in hits.iter().enumerate() {
            let obj = serde_json::json!({
                "doc_id": hit.doc_id,
                "doc_path": hit.doc_path,
                "doc_title": hit.doc_title,
                "snippet": hit.snippet,
                "line_start": hit.line_start,
                "line_end": hit.line_end,
                "score": hit.score,
            });
            buf.push_str(&serde_json::to_string(&obj)?);
            if i + 1 < hits.len() {
                buf.push(',');
            }
            buf.push('\n');
        }
        buf.push(']');
        println!("{}", buf);
    } else {
        for (i, hit) in hits.iter().enumerate() {
            println!(
                "\n  [{}] {} (lines {}-{})",
                i + 1,
                hit.doc_title,
                hit.line_start,
                hit.line_end
            );
            println!("       {}", hit.snippet);
            println!("       path: {}", hit.doc_path);
            println!("       bm25: {:.4}", hit.score);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::ingest::ingest_file;
    use std::io::Write;

    fn isolated_home() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        std::env::set_var("IMPFORGE_CLI_HOME", dir.path());
        dir
    }

    fn write_temp_text(content: &str) -> tempfile::NamedTempFile {
        let tmp = tempfile::NamedTempFile::with_suffix(".txt").expect("tmp");
        tmp.as_file().write_all(content.as_bytes()).expect("write");
        tmp
    }

    #[test]
    fn search_finds_ingested_keyword() {
        let _home = isolated_home();
        let db = knowledge_db_path().expect("db");
        let mut conn = open_db(&db).expect("open");

        let f = write_temp_text(
            "The quick brown fox jumps over the lazy dog.\n\
             Reciprocal rank fusion rocks for hybrid retrieval.\n\
             The impforge knowledge base loves bilingual search.",
        );
        ingest_file(&mut conn, f.path(), &[], false)
            .expect("ingest")
            .expect("recognised");

        let hits = run_query(&conn, "fox", 10).expect("search");
        assert_eq!(hits.len(), 1);
        assert!(hits[0].snippet.contains("fox") || hits[0].snippet.contains("<<fox>>"));
        assert!(hits[0].doc_id > 0);
    }

    #[test]
    fn search_returns_empty_for_unknown_term() {
        let _home = isolated_home();
        let db = knowledge_db_path().expect("db");
        let mut conn = open_db(&db).expect("open");

        let f = write_temp_text("only the basic content here.");
        ingest_file(&mut conn, f.path(), &[], false)
            .expect("ingest")
            .expect("recognised");

        let hits = run_query(&conn, "supercalifragilistic", 10).expect("search");
        assert!(hits.is_empty());
    }

    #[test]
    fn search_ranks_more_specific_match_higher() {
        let _home = isolated_home();
        let db = knowledge_db_path().expect("db");
        let mut conn = open_db(&db).expect("open");

        // Doc 1 mentions "rust" once.
        let f1 = write_temp_text(
            "the impforge cli is implemented in rust for cross-platform reach.\n\
             documentation is markdown-friendly.",
        );
        ingest_file(&mut conn, f1.path(), &[], false)
            .expect("ingest 1")
            .expect("recognised");

        // Doc 2 mentions "rust" three times in the same chunk → BM25
        // ranks it higher.
        let f2 = write_temp_text(
            "rust rust rust is the common keyword across this short blob.\n\
             it should rank higher for the rust query.",
        );
        ingest_file(&mut conn, f2.path(), &[], false)
            .expect("ingest 2")
            .expect("recognised");

        let hits = run_query(&conn, "rust", 10).expect("search");
        assert!(hits.len() >= 2, "expected at least 2 hits, got {}", hits.len());

        // FTS5's bm25() returns negative scores; lower = more relevant.
        // Top result must be doc 2.
        let top_path = &hits[0].doc_path;
        let f2_path = f2
            .path()
            .canonicalize()
            .expect("canon")
            .to_string_lossy()
            .to_string();
        assert_eq!(*top_path, f2_path, "doc with 3x 'rust' must rank first");
    }

    #[test]
    fn search_supports_phrase_query() {
        let _home = isolated_home();
        let db = knowledge_db_path().expect("db");
        let mut conn = open_db(&db).expect("open");

        let f = write_temp_text(
            "Reciprocal rank fusion is a well-known IR primitive.\n\
             Users typically combine BM25 and vector cosine before fusion.",
        );
        ingest_file(&mut conn, f.path(), &[], false)
            .expect("ingest")
            .expect("recognised");

        // FTS5 phrase-match syntax is "double quotes inside the SQL
        // bind", so we wrap.
        let hits = run_query(&conn, "\"rank fusion\"", 10).expect("phrase");
        assert_eq!(hits.len(), 1);
    }
}
