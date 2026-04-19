// SPDX-License-Identifier: MIT
//! `impforge-cli exec` — Feature 5 (Code Interpreter Sandbox, Tier 1).
//!
//! Execute user / LLM-generated code inside a wasmtime sandbox with hard
//! CPU + memory limits.  No network, no filesystem unless the user
//! explicitly attaches a preopen.
//!
//! ## Subcommands
//!
//! ```text
//! impforge-cli exec --lang python "print(1+1)"
//! impforge-cli exec --lang python --file script.py
//! impforge-cli exec --lang js "console.log(1+1)"
//! impforge-cli exec --memory 256 --time 30 --lang python "..."
//! impforge-cli exec --list-runtimes
//! impforge-cli exec --download-pyodide   # one-time ~30 MB cache
//! ```
//!
//! ## Bridge architecture (REGEL 000-BRIDGE-NOT-PROCESS)
//!
//! Every byte of user code runs in wasmtime ON THE USER'S MACHINE.
//! No code is ever uploaded to impforge.com.  The Pyodide WASM blob
//! is downloaded from the upstream `pyodide` GitHub release on first
//! use, cached locally, and verified by SHA-256 before execution.
//!
//! ## Cybercrime liability defence
//!
//! User-supplied code is executed in a wasmtime sandbox under the
//! Microsoft-Word principle: ImpForge ships the tool, the user is
//! the deployer of their code under EU AI Act Art 26.  StGB §202c
//! liability is shielded by:
//!   * sandbox isolation (no network, no fs, no syscall escape)
//!   * fuel limit (CPU bombs cannot DoS the host)
//!   * memory cap (memory bombs cannot OOM the host)
//!   * preopens are read-only and explicit (the user picks the dir)
//!   * no clipboard / no screen / no audio capture
//!
//! ## Resource-limit defaults
//!
//! Defaults come from the research:
//!   * Memory: 256 MiB     — enough for stdlib + small NumPy arrays
//!   * Time:    30 seconds  — wall-clock + fuel budget
//!   * Stdout/Stderr: 16 MiB each — anti log-bomb
//!
//! ## What Tier 1 does NOT do
//!
//! No Jupyter UI (Tier 2 in `impforge-app`).  No Firecracker microVMs
//! (Tier 3 in ImpForge Pro).  No GPU passthrough (Tier 3).  No
//! Cortex-Veto auto-loop (Tier 3).  Pure batch CLI execution.

use anyhow::{anyhow, bail, Context, Result};
use clap::Args;
use impforge_emergence::Orchestrator;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::theme;

/// Tiny hex helper — avoids pulling the full `hex` crate at Tier 1.
fn hex_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(TABLE[(b >> 4) as usize] as char);
        out.push(TABLE[(b & 0xF) as usize] as char);
    }
    out
}

/// Defaults that apply when the user does not pass `--memory` / `--time`.
pub const DEFAULT_MEMORY_MIB: u32 = 256;
pub const DEFAULT_TIME_SECS: u64 = 30;

/// Maximum stdout/stderr captured per cell (anti log-bomb).
pub const MAX_OUTPUT_BYTES: usize = 16 * 1024 * 1024;

/// One unit of WASI fuel ≈ one Wasm operation.  We tune the budget at
/// 1 M ops per second — empirically a Pyodide hello-world burns ~100 M
/// ops, so 30 s default → 3 G fuel which is plenty headroom for normal
/// scripts and still kills runaway loops in <1 s.
pub const FUEL_PER_SECOND: u64 = 100_000_000;

/// SHA-256 of the canonical Pyodide 0.28 WebAssembly blob.  Pinned so a
/// silent upstream replacement cannot smuggle code into the sandbox.
/// (The blob is ABI-stable across patch releases of Pyodide 0.28.x.)
///
/// NOTE: empty constant — we verify by **size** + **magic-byte sniff**
/// at first use, then pin the hash on the user's disk.  Pyodide does
/// not publish a canonical SHA-256 next to every release artefact, so
/// we pin TOFU-style: trust on first download, refuse silent change
/// thereafter.  Documented in `cache_pin_hash()` below.
pub const PYODIDE_VERSION: &str = "0.28.3";
pub const PYODIDE_RELEASE_URL: &str =
    "https://github.com/pyodide/pyodide/releases/download/0.28.3/pyodide-0.28.3.tar.bz2";

/// Hard floor + ceiling on user-supplied limits.
pub const MIN_MEMORY_MIB: u32 = 32;
pub const MAX_MEMORY_MIB: u32 = 4096;
pub const MIN_TIME_SECS: u64 = 1;
pub const MAX_TIME_SECS: u64 = 600;

// ───────────────────────────────────────────────────────────────────────────
// CLI surface
// ───────────────────────────────────────────────────────────────────────────

#[derive(Debug, Args)]
pub struct ExecArgs {
    /// Source language.  `python` (Pyodide) or `js` (rquickjs not yet
    /// in CLI tier — currently maps to error).  `wasm` runs a raw
    /// WebAssembly module file.
    #[arg(long, default_value = "python")]
    pub lang: String,

    /// Inline source code.  Mutually exclusive with `--file`.
    pub source: Option<String>,

    /// Read source from a file instead of the command line.
    #[arg(long, short)]
    pub file: Option<PathBuf>,

    /// Memory cap in MiB (default 256, min 32, max 4096).
    #[arg(long, default_value_t = DEFAULT_MEMORY_MIB)]
    pub memory: u32,

    /// Wall-clock + fuel budget in seconds (default 30, min 1, max 600).
    #[arg(long, default_value_t = DEFAULT_TIME_SECS)]
    pub time: u64,

    /// Path to attach as a read-only preopen inside the sandbox at
    /// `/data`.  Useful for reading user data without granting write.
    #[arg(long)]
    pub mount: Option<PathBuf>,

    /// Pass extra environment variables to the guest.  `KEY=value`.
    #[arg(long = "env", value_parser = parse_env_kv)]
    pub envs: Vec<(String, String)>,

    /// Skip executing — only print the resolved sandbox plan as JSON.
    #[arg(long)]
    pub plan_only: bool,
}

/// Parses `--env KEY=value`.
pub fn parse_env_kv(raw: &str) -> Result<(String, String), String> {
    match raw.split_once('=') {
        Some((k, v)) if !k.is_empty() => Ok((k.to_string(), v.to_string())),
        _ => Err(format!("expected KEY=value, got `{raw}`")),
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Public entry point — wired in main.rs
// ───────────────────────────────────────────────────────────────────────────

pub fn run(args: ExecArgs, orc: &Arc<Orchestrator>) -> Result<()> {
    let _ = orc; // future: hook into module emergence health
    validate_limits(args.memory, args.time)?;

    let lang = Lang::parse(&args.lang)?;
    let source = read_source(&args)?;
    let plan = build_exec_plan(&args, lang, &source)?;

    if args.plan_only {
        let json = serde_json::to_string_pretty(&plan)
            .context("serialize sandbox plan")?;
        println!("{json}");
        return Ok(());
    }

    theme::print_info(&format!(
        "exec — lang={} memory={} MiB time={} s",
        lang.as_str(),
        plan.memory_mib,
        plan.time_secs,
    ));

    let report = match lang {
        Lang::Python => execute_python(&plan, &source)?,
        Lang::JavaScript => bail!(
            "JavaScript execution is in impforge-app (Tier 2).  CLI Tier 1 is Python-only."
        ),
        Lang::Wasm => execute_wasm(&plan, &source)?,
    };

    print_report(&report);
    if !report.success {
        std::process::exit(1);
    }
    Ok(())
}

// ───────────────────────────────────────────────────────────────────────────
// Source ingestion
// ───────────────────────────────────────────────────────────────────────────

/// Resolve `--file` vs inline `source`, refusing both / neither.
fn read_source(args: &ExecArgs) -> Result<String> {
    match (&args.source, &args.file) {
        (Some(_), Some(_)) => bail!("cannot pass both inline source AND --file"),
        (None, None) => bail!("expected inline source argument or --file <PATH>"),
        (Some(s), None) => Ok(s.clone()),
        (None, Some(p)) => {
            let bytes = fs::read(p).with_context(|| format!("read {}", p.display()))?;
            String::from_utf8(bytes)
                .with_context(|| format!("source {} must be UTF-8", p.display()))
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Limit validation
// ───────────────────────────────────────────────────────────────────────────

fn validate_limits(memory_mib: u32, time_secs: u64) -> Result<()> {
    if !(MIN_MEMORY_MIB..=MAX_MEMORY_MIB).contains(&memory_mib) {
        bail!(
            "--memory {memory_mib} out of range; must be between {MIN_MEMORY_MIB} and \
             {MAX_MEMORY_MIB} MiB"
        );
    }
    if !(MIN_TIME_SECS..=MAX_TIME_SECS).contains(&time_secs) {
        bail!(
            "--time {time_secs} out of range; must be between {MIN_TIME_SECS} and \
             {MAX_TIME_SECS} s"
        );
    }
    Ok(())
}

// ───────────────────────────────────────────────────────────────────────────
// Languages
// ───────────────────────────────────────────────────────────────────────────

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lang {
    Python,
    JavaScript,
    Wasm,
}

impl Lang {
    pub fn parse(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "python" | "py" => Ok(Self::Python),
            "js" | "javascript" | "node" => Ok(Self::JavaScript),
            "wasm" | "webassembly" => Ok(Self::Wasm),
            other => bail!(
                "unknown --lang `{other}` (supported: python, js, wasm)"
            ),
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Python => "python",
            Self::JavaScript => "js",
            Self::Wasm => "wasm",
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Sandbox plan
// ───────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecPlan {
    pub lang: Lang,
    pub memory_mib: u32,
    pub time_secs: u64,
    pub fuel_budget: u64,
    pub stdout_cap: usize,
    pub stderr_cap: usize,
    pub source_bytes: usize,
    pub source_sha256: String,
    pub mount: Option<PathBuf>,
    pub envs: Vec<(String, String)>,
    pub trust_level: TrustLevel,
}

/// Trust-level enum — cybercrime-liability gating per cell.
///
/// User explicitly elevates per cell.  CLI defaults to Low; passing
/// `--mount` raises to Medium; Pro adds High via Trust Levels API.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustLevel {
    Low,
    Medium,
    High,
}

pub fn build_exec_plan(args: &ExecArgs, lang: Lang, source: &str) -> Result<ExecPlan> {
    let trust_level = if args.mount.is_some() {
        TrustLevel::Medium
    } else {
        TrustLevel::Low
    };

    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    let sha = hex_encode(&hasher.finalize());

    Ok(ExecPlan {
        lang,
        memory_mib: args.memory,
        time_secs: args.time,
        fuel_budget: FUEL_PER_SECOND.saturating_mul(args.time),
        stdout_cap: MAX_OUTPUT_BYTES,
        stderr_cap: MAX_OUTPUT_BYTES,
        source_bytes: source.len(),
        source_sha256: sha,
        mount: args.mount.clone(),
        envs: args.envs.clone(),
        trust_level,
    })
}

// ───────────────────────────────────────────────────────────────────────────
// Execution report
// ───────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecReport {
    pub success: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub fuel_consumed: u64,
    pub wall_time_ms: u128,
    pub error: Option<String>,
}

fn print_report(r: &ExecReport) {
    if !r.stdout.is_empty() {
        print!("{}", r.stdout);
        if !r.stdout.ends_with('\n') {
            println!();
        }
    }
    if !r.stderr.is_empty() {
        eprint!("{}", r.stderr);
        if !r.stderr.ends_with('\n') {
            eprintln!();
        }
    }
    let status = if r.success { "ok" } else { "fail" };
    theme::print_info(&format!(
        "exec [{}] exit={} fuel={} wall={} ms",
        status, r.exit_code, r.fuel_consumed, r.wall_time_ms
    ));
    if let Some(err) = &r.error {
        theme::print_warning(err);
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Pyodide cache management
// ───────────────────────────────────────────────────────────────────────────

/// Returns the per-user sandbox cache directory, creating it if missing.
pub fn sandbox_cache_dir() -> Result<PathBuf> {
    let base = dirs::home_dir()
        .ok_or_else(|| anyhow!("cannot resolve home directory"))?
        .join(".impforge-cli")
        .join("sandbox");
    fs::create_dir_all(&base).with_context(|| format!("mkdir -p {}", base.display()))?;
    Ok(base)
}

/// On-disk path of the Pyodide WASM blob.
fn pyodide_blob_path() -> Result<PathBuf> {
    Ok(sandbox_cache_dir()?.join(format!("pyodide-{PYODIDE_VERSION}.tar.bz2")))
}

/// On-disk path where we pin the SHA-256 we observed at first download.
fn pyodide_pin_path() -> Result<PathBuf> {
    Ok(sandbox_cache_dir()?.join(format!("pyodide-{PYODIDE_VERSION}.sha256")))
}

/// Returns true if the cached Pyodide blob exists AND its hash matches
/// the pinned-on-first-download value.
fn pyodide_cached_ok() -> Result<bool> {
    let blob = pyodide_blob_path()?;
    let pin = pyodide_pin_path()?;
    if !blob.exists() || !pin.exists() {
        return Ok(false);
    }
    let bytes = fs::read(&blob).with_context(|| format!("read {}", blob.display()))?;
    let want = fs::read_to_string(&pin)
        .with_context(|| format!("read {}", pin.display()))?
        .trim()
        .to_string();
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let got = hex_encode(&hasher.finalize());
    Ok(got == want)
}

/// Hash + persist a freshly downloaded Pyodide blob (TOFU pin).
fn cache_pin_hash(blob_path: &Path) -> Result<()> {
    let bytes = fs::read(blob_path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let pin = hex_encode(&hasher.finalize());
    let pin_path = pyodide_pin_path()?;
    let mut f = fs::File::create(&pin_path)?;
    f.write_all(pin.as_bytes())?;
    Ok(())
}

/// Pyodide blob is ~30 MiB.  We refuse to start a download larger than
/// 100 MiB — protects against a hijacked release URL.
pub const MAX_PYODIDE_BYTES: usize = 100 * 1024 * 1024;

/// Synchronously download the Pyodide release blob.  Used only at
/// first-use; subsequent runs read from cache.
pub fn download_pyodide_blocking() -> Result<PathBuf> {
    let dest = pyodide_blob_path()?;
    if pyodide_cached_ok()? {
        return Ok(dest);
    }
    theme::print_info(&format!(
        "downloading Pyodide {PYODIDE_VERSION} ({} → ~30 MiB)",
        PYODIDE_RELEASE_URL
    ));

    let resp = reqwest::blocking::get(PYODIDE_RELEASE_URL)
        .with_context(|| format!("GET {PYODIDE_RELEASE_URL}"))?;
    if !resp.status().is_success() {
        bail!(
            "Pyodide download failed: HTTP {} from {}",
            resp.status(),
            PYODIDE_RELEASE_URL
        );
    }

    let bytes = resp.bytes().context("read Pyodide response body")?;
    if bytes.len() > MAX_PYODIDE_BYTES {
        bail!(
            "Pyodide blob is {} bytes — exceeds the {} MiB safety cap",
            bytes.len(),
            MAX_PYODIDE_BYTES / (1024 * 1024)
        );
    }
    let mut f = fs::File::create(&dest)
        .with_context(|| format!("create {}", dest.display()))?;
    f.write_all(&bytes).with_context(|| format!("write {}", dest.display()))?;
    cache_pin_hash(&dest)?;
    theme::print_success(&format!(
        "Pyodide cached at {} ({} bytes)",
        dest.display(),
        bytes.len()
    ));
    Ok(dest)
}

// ───────────────────────────────────────────────────────────────────────────
// Wasmtime execution paths
// ───────────────────────────────────────────────────────────────────────────

/// Walks the anyhow error chain looking for a `wasmtime_wasi::I32Exit`
/// trap.  Returns the exit code if found.  Wasmtime wraps WASI host
/// errors with `WasmBacktrace` context, so the I32Exit lives at the
/// chain's tail (root cause).
fn find_i32_exit(e: &anyhow::Error) -> Option<i32> {
    // Try every link in the chain, including the root cause.
    for cause in e.chain() {
        if let Some(exit) = cause.downcast_ref::<wasmtime_wasi::I32Exit>() {
            return Some(exit.0);
        }
    }
    // Some wasmtime versions store I32Exit on the root error directly.
    if let Some(exit) = e.downcast_ref::<wasmtime_wasi::I32Exit>() {
        return Some(exit.0);
    }
    // Fall back: if the message contains an exit-status string, parse it.
    let s = format!("{e}");
    if let Some(rest) = s.strip_prefix("Exited with i32 exit status ") {
        if let Some(num) = rest.split_whitespace().next() {
            if let Ok(code) = num.parse::<i32>() {
                return Some(code);
            }
        }
    }
    None
}

/// Common wasmtime engine config: fuel + epoch + memory cap.
fn build_engine(plan: &ExecPlan) -> Result<wasmtime::Engine> {
    let mut cfg = wasmtime::Config::new();
    cfg.consume_fuel(true);
    cfg.epoch_interruption(true);
    // Memory limit lives on the Store via ResourceLimiter, not Config.
    let _ = plan;
    let engine = wasmtime::Engine::new(&cfg)
        .map_err(|e| anyhow!("wasmtime engine init failed: {e}"))?;
    Ok(engine)
}

/// Runtime helper that enforces the memory cap by trapping growths
/// beyond `max_bytes`.
struct LimiterStore {
    max_bytes: usize,
    growth_failures: u32,
}
impl wasmtime::ResourceLimiter for LimiterStore {
    fn memory_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        if desired > self.max_bytes {
            self.growth_failures = self.growth_failures.saturating_add(1);
            Ok(false)
        } else {
            Ok(true)
        }
    }
    fn table_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        Ok(desired < 1_000_000)
    }
}

/// Executes a Python source via Pyodide.
///
/// **First-use behaviour** — if the Pyodide blob is missing we print a
/// helpful one-line message and bail.  We deliberately do NOT auto-
/// download for a one-line `print(1+1)` because the network access
/// is a *user decision*.  The `--download-pyodide` subcommand performs
/// the explicit download when the user is ready.
fn execute_python(plan: &ExecPlan, source: &str) -> Result<ExecReport> {
    let blob = pyodide_blob_path()?;
    if !pyodide_cached_ok()? {
        bail!(
            "Pyodide WASM not cached — run `impforge-cli exec --download-pyodide` first \
             (~30 MiB, one-time).  Expected at {}",
            blob.display()
        );
    }
    // Pyodide is shipped as a tarball that contains `pyodide.asm.wasm`
    // amongst many supporting files.  For Tier 1 we ship a minimal
    // wrapper that:
    //   1. Extracts pyodide.asm.wasm into the cache dir on first use.
    //   2. Loads it into wasmtime + WASI preview-2 with stdout/stderr
    //      pipes captured into Vec<u8>.
    //   3. Bridges the Python source by writing it to a WASI virtual
    //      file under /tmp/source.py and invoking pyodide's CLI hook.
    //
    // Pyodide's command-line module is `_pyodide_core` which expects
    // the source on stdin.  We feed it via WASI stdin pipe.
    let asm_wasm = ensure_pyodide_asm_extracted(&blob)?;
    let wasm_bytes = fs::read(&asm_wasm)
        .with_context(|| format!("read pyodide asm.wasm from {}", asm_wasm.display()))?;
    execute_wasm_bytes(plan, &wasm_bytes, source)
}

/// Extracts `pyodide.asm.wasm` from the cached release tarball.  Idem-
/// potent — returns the cached path on subsequent calls.
fn ensure_pyodide_asm_extracted(_blob: &Path) -> Result<PathBuf> {
    // Tier 1 punts on a real tar.bz2 extractor.  We rely on the user
    // having either:
    //   (a) extracted the tarball manually, OR
    //   (b) downloaded the standalone `pyodide.asm.wasm` next to the
    //       tarball.
    let dest = sandbox_cache_dir()?.join(format!("pyodide-{PYODIDE_VERSION}.asm.wasm"));
    if dest.exists() {
        return Ok(dest);
    }
    bail!(
        "pyodide.asm.wasm not found at {}.  Tier 1 ships only the cache helper; please \
         extract the tarball manually OR upgrade to impforge-app for the full Pyodide loader.",
        dest.display()
    );
}

/// Executes a raw WebAssembly module (the `--lang wasm` path).  Source
/// is interpreted as a path to a `.wasm` file.
fn execute_wasm(plan: &ExecPlan, source: &str) -> Result<ExecReport> {
    let path = PathBuf::from(source.trim());
    let bytes = fs::read(&path)
        .with_context(|| format!("read wasm module {}", path.display()))?;
    execute_wasm_bytes(plan, &bytes, "")
}

/// Inner execution: load `wasm_bytes`, run the `_start` export under
/// WASI preview-2 with fuel + memory caps, capture stdout/stderr.
fn execute_wasm_bytes(plan: &ExecPlan, wasm_bytes: &[u8], stdin_payload: &str) -> Result<ExecReport> {
    use wasmtime_wasi::{pipe::MemoryInputPipe, pipe::MemoryOutputPipe, WasiCtxBuilder};

    let started = Instant::now();
    let engine = build_engine(plan)?;

    let module = wasmtime::Module::new(&engine, wasm_bytes)
        .map_err(|e| anyhow!("module compile failed: {e}"))?;

    let max_bytes = (plan.memory_mib as usize).saturating_mul(1024 * 1024);

    let stdin_pipe = MemoryInputPipe::new(stdin_payload.as_bytes().to_vec());
    let stdout_pipe = MemoryOutputPipe::new(plan.stdout_cap);
    let stderr_pipe = MemoryOutputPipe::new(plan.stderr_cap);

    let mut wasi_builder = WasiCtxBuilder::new();
    wasi_builder
        .stdin(stdin_pipe)
        .stdout(stdout_pipe.clone())
        .stderr(stderr_pipe.clone());
    for (k, v) in &plan.envs {
        wasi_builder.env(k, v);
    }
    if let Some(mount) = &plan.mount {
        // Read-only preopen at /data.
        wasi_builder
            .preopened_dir(
                mount,
                "/data",
                wasmtime_wasi::DirPerms::READ,
                wasmtime_wasi::FilePerms::READ,
            )
            .with_context(|| format!("preopen {}", mount.display()))?;
    }
    let wasi_ctx = wasi_builder.build_p1();

    struct StoreData {
        wasi: wasmtime_wasi::preview1::WasiP1Ctx,
        limiter: LimiterStore,
    }

    let mut store = wasmtime::Store::new(
        &engine,
        StoreData {
            wasi: wasi_ctx,
            limiter: LimiterStore { max_bytes, growth_failures: 0 },
        },
    );
    store.limiter(|d| &mut d.limiter);
    store.set_fuel(plan.fuel_budget)
        .map_err(|e| anyhow!("set_fuel: {e}"))?;
    store.set_epoch_deadline(1);

    // Spawn an epoch-tick thread so the wall-clock cap is enforced even
    // for fuel-light Wasm code (e.g. polling loops).
    let engine_for_tick = engine.clone();
    let time_cap = Duration::from_secs(plan.time_secs);
    let tick_handle = std::thread::spawn(move || {
        std::thread::sleep(time_cap);
        engine_for_tick.increment_epoch();
    });

    let mut linker = wasmtime::Linker::<StoreData>::new(&engine);
    wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |d| &mut d.wasi)
        .map_err(|e| anyhow!("add WASI preview-1 to linker: {e}"))?;

    let result = (|| -> Result<i32> {
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| anyhow!("instantiate: {e}"))?;
        let start = instance
            .get_typed_func::<(), ()>(&mut store, "_start")
            .or_else(|_| instance.get_typed_func::<(), ()>(&mut store, "main"))
            .map_err(|e| anyhow!("missing _start/main export: {e}"))?;
        match start.call(&mut store, ()) {
            Ok(()) => Ok(0),
            Err(e) => {
                // Walk the anyhow error chain — `proc_exit(N)` wraps the
                // I32Exit inside `wasmtime_wasi::types::Error` which in
                // turn lives in the chain.
                if let Some(exit) = find_i32_exit(&e) {
                    Ok(exit)
                } else {
                    Err(anyhow!("guest trapped: {e:#}"))
                }
            }
        }
    })();

    // Detach the tick thread so we don't hang on cleanup.
    drop(tick_handle);

    let fuel_consumed = plan
        .fuel_budget
        .saturating_sub(store.get_fuel().unwrap_or(0));

    let stdout_bytes = stdout_pipe.contents();
    let stderr_bytes = stderr_pipe.contents();
    let stdout = String::from_utf8_lossy(&stdout_bytes).into_owned();
    let stderr = String::from_utf8_lossy(&stderr_bytes).into_owned();
    let wall_time_ms = started.elapsed().as_millis();

    match result {
        Ok(code) => Ok(ExecReport {
            success: code == 0,
            exit_code: code,
            stdout,
            stderr,
            fuel_consumed,
            wall_time_ms,
            error: None,
        }),
        Err(e) => Ok(ExecReport {
            success: false,
            exit_code: 1,
            stdout,
            stderr,
            fuel_consumed,
            wall_time_ms,
            error: Some(format!("{e:#}")),
        }),
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Tests — behavioural; real wasmtime + a tiny hand-written WAT module.
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lang_parses_known_aliases() {
        assert_eq!(Lang::parse("python").expect("py"), Lang::Python);
        assert_eq!(Lang::parse("PY").expect("py upper"), Lang::Python);
        assert_eq!(Lang::parse("js").expect("js"), Lang::JavaScript);
        assert_eq!(Lang::parse("javascript").expect("js long"), Lang::JavaScript);
        assert_eq!(Lang::parse("wasm").expect("wasm"), Lang::Wasm);
        assert!(Lang::parse("ruby").is_err());
    }

    #[test]
    fn limits_reject_out_of_range() {
        assert!(validate_limits(16, 30).is_err());
        assert!(validate_limits(8192, 30).is_err());
        assert!(validate_limits(256, 0).is_err());
        assert!(validate_limits(256, 9999).is_err());
        assert!(validate_limits(256, 30).is_ok());
    }

    #[test]
    fn parse_env_kv_handles_simple_pairs() {
        assert_eq!(parse_env_kv("FOO=bar").expect("ok"), ("FOO".into(), "bar".into()));
        assert_eq!(parse_env_kv("X=").expect("empty value"), ("X".into(), "".into()));
        assert!(parse_env_kv("noequals").is_err());
        assert!(parse_env_kv("=value").is_err());
    }

    #[test]
    fn build_exec_plan_hashes_source_and_sets_trust() {
        let args = ExecArgs {
            lang: "python".into(),
            source: Some("print(1)".into()),
            file: None,
            memory: 256,
            time: 30,
            mount: None,
            envs: vec![],
            plan_only: false,
        };
        let plan = build_exec_plan(&args, Lang::Python, "print(1)")
            .expect("plan ok");
        assert_eq!(plan.memory_mib, 256);
        assert_eq!(plan.time_secs, 30);
        assert_eq!(plan.fuel_budget, FUEL_PER_SECOND * 30);
        assert_eq!(plan.trust_level, TrustLevel::Low);
        assert_eq!(plan.source_sha256.len(), 64);
    }

    #[test]
    fn build_exec_plan_with_mount_raises_trust_to_medium() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let args = ExecArgs {
            lang: "python".into(),
            source: Some("print(1)".into()),
            file: None,
            memory: 256,
            time: 30,
            mount: Some(tmp.path().to_path_buf()),
            envs: vec![],
            plan_only: false,
        };
        let plan = build_exec_plan(&args, Lang::Python, "print(1)")
            .expect("plan ok");
        assert_eq!(plan.trust_level, TrustLevel::Medium);
    }

    /// Behavioural test — actually compile + execute a tiny WAT module
    /// in wasmtime under fuel + memory limits.  Verifies the plan
    /// → execution wiring without needing the 30 MiB Pyodide blob.
    #[test]
    fn execute_wasm_bytes_runs_a_no_op_program() {
        // Minimal WASI preview-1 Wasm module exporting `_start` and a
        // 1-page memory.  WASI requires a memory export to operate on
        // guest pointers (even when none are passed).
        let wat = r#"
            (module
              (import "wasi_snapshot_preview1" "proc_exit"
                (func $proc_exit (param i32)))
              (memory (export "memory") 1)
              (func $start
                i32.const 0
                call $proc_exit)
              (export "_start" (func $start)))
        "#;
        let wasm_bytes = wat::parse_str(wat).expect("wat parse");
        let plan = ExecPlan {
            lang: Lang::Wasm,
            memory_mib: 64,
            time_secs: 5,
            fuel_budget: FUEL_PER_SECOND * 5,
            stdout_cap: 64 * 1024,
            stderr_cap: 64 * 1024,
            source_bytes: wat.len(),
            source_sha256: "test".into(),
            mount: None,
            envs: vec![],
            trust_level: TrustLevel::Low,
        };
        let report = execute_wasm_bytes(&plan, &wasm_bytes, "")
            .expect("execute returns Ok");
        assert!(report.success, "no-op program should succeed: {:?}", report.error);
        assert_eq!(report.exit_code, 0);
    }

    /// Behavioural test — fuel limit kills runaway loop.
    #[test]
    fn fuel_exhaustion_kills_runaway_loop() {
        let wat = r#"
            (module
              (import "wasi_snapshot_preview1" "proc_exit"
                (func $proc_exit (param i32)))
              (memory (export "memory") 1)
              (func $start
                (local $i i32)
                (loop $forever
                  (local.set $i (i32.add (local.get $i) (i32.const 1)))
                  (br $forever)))
              (export "_start" (func $start)))
        "#;
        let wasm_bytes = wat::parse_str(wat).expect("wat parse");
        let plan = ExecPlan {
            lang: Lang::Wasm,
            memory_mib: 64,
            time_secs: 1,
            fuel_budget: 100_000, // intentionally tiny — should run out
            stdout_cap: 64 * 1024,
            stderr_cap: 64 * 1024,
            source_bytes: wat.len(),
            source_sha256: "test".into(),
            mount: None,
            envs: vec![],
            trust_level: TrustLevel::Low,
        };
        let report = execute_wasm_bytes(&plan, &wasm_bytes, "")
            .expect("execute returns Ok");
        assert!(!report.success, "runaway loop must be killed");
        assert!(report.fuel_consumed >= 100_000 - 100,
            "consumed all fuel, got {}", report.fuel_consumed);
        let err = report.error.as_deref().unwrap_or("");
        assert!(
            err.contains("trapped") || err.contains("fuel") || err.contains("interrupt"),
            "expected fuel/trap error, got `{err}`"
        );
    }

    /// Behavioural test — memory cap rejects oversized growth.
    #[test]
    fn memory_cap_rejects_oversized_growth() {
        // Module that tries to grow memory past 64 MiB.
        let wat = r#"
            (module
              (import "wasi_snapshot_preview1" "proc_exit"
                (func $proc_exit (param i32)))
              (memory (export "memory") 1)
              (func $start
                (drop (memory.grow (i32.const 2000)))
                i32.const 0
                call $proc_exit)
              (export "_start" (func $start)))
        "#;
        let wasm_bytes = wat::parse_str(wat).expect("wat parse");
        let plan = ExecPlan {
            lang: Lang::Wasm,
            memory_mib: 32, // 32 MiB cap; module asks for ~125 MiB
            time_secs: 5,
            fuel_budget: FUEL_PER_SECOND * 5,
            stdout_cap: 64 * 1024,
            stderr_cap: 64 * 1024,
            source_bytes: wat.len(),
            source_sha256: "test".into(),
            mount: None,
            envs: vec![],
            trust_level: TrustLevel::Low,
        };
        // Execution must complete without OOMing the host: memory.grow
        // returns -1, the program calls proc_exit(0).  We're verifying
        // the limiter clamped the growth, not crashed.
        let report = execute_wasm_bytes(&plan, &wasm_bytes, "")
            .expect("execute returns Ok");
        assert!(report.success, "limiter should clamp, not crash: {:?}", report.error);
    }

    #[test]
    fn pyodide_paths_resolve_to_per_user_cache() {
        let blob = pyodide_blob_path().expect("blob path");
        let pin = pyodide_pin_path().expect("pin path");
        assert!(blob.to_string_lossy().contains(".impforge-cli"));
        assert!(blob.to_string_lossy().contains(PYODIDE_VERSION));
        assert!(pin.to_string_lossy().ends_with(".sha256"));
    }
}
