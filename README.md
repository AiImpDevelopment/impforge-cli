# impforge-cli

<p align="center">
  <img src="https://img.shields.io/badge/license-MIT-green" alt="MIT" />
  <img src="https://img.shields.io/badge/built--with-Rust-orange" alt="Rust" />
  <img src="https://img.shields.io/badge/MCP-native-blueviolet" alt="MCP-native" />
  <img src="https://img.shields.io/badge/templates-78-blue" alt="78 templates" />
  <img src="https://img.shields.io/badge/compliance--rules-2600-brightgreen" alt="2 600 rules" />
  <img src="https://img.shields.io/badge/local--models-Ollama%20%C2%B7%20HF%20%C2%B7%20llama.cpp%20%C2%B7%20Candle-black" alt="Local models" />
</p>

<h3 align="center">
  The MCP-native AI coding companion.<br>
  78 production-grade industry templates · 2 600 compliance rules · works with every AI tool · local-model-first.
</h3>

---

## One-line install

```bash
cargo install impforge-cli
# or
brew install impforge-cli
# or
scoop install impforge-cli
```

## 30-second tour

```bash
# Browse the 78 industry-template scaffolds.
impforge-cli template list

# Scaffold a FinTech multi-tenant SaaS locally.
impforge-cli template scaffold fintech-saas ./my-fintech

# Wire us into your AI coding tool of choice (Cursor / Claude Code / Lovable / Bolt / Windsurf / Zed).
impforge-cli mcp register claude-code

# Pull THE BRAIN — the exact 8B model that powers ImpForge Pro, locally.
impforge-cli brain pull

# Generate a project from a template + local model (100 % offline).
impforge-cli generate --template healthcare-saas --model brain

# Diagnose + self-heal.
impforge-cli doctor

# Beautiful futuristic TUI dashboard.
impforge-cli tui            # requires `--features tui`
```

## What you get (MIT, free forever)

| Feature | What | Count |
|---------|------|-------|
| **Templates** | Production scaffolds for 26 industries × 3 categories | 78 |
| **Compliance rules** | Real regulatory rules with citation, severity, enforcement | 2 600 |
| **Skills** | Reusable AI-coding recipes | growing |
| **MCP manifests** | 20+ curated MCP servers + ours | 23+ |
| **Local-model backends** | Ollama · HuggingFace Hub · llama.cpp · Candle | 4 |
| **Self-update** | `cargo install`-aware auto-check | ✓ |
| **Self-heal** | Doctor command detects + repairs | ✓ |
| **Autopilot** | Opt-in background daemon | ✓ |
| **Contribution wizard** | Local validation + GitHub PR builder | ✓ |

## What you DON'T get (upgrade to ImpForge Pro — € 25/mo)

- Pro Mesh (connects templates × 157 870 quality rules × 50 K intent patterns × 12.4 K SaaS tools × 472 personas × 40 agents × deep-links)
- 4-Model Collaboration Pipeline (THE BRAIN alone in free; classifier + fast-hands + embedder in Pro)
- Offline AI workstation with Live Preview + Pop-out windows
- Tenant-Guard live auditing
- Signed-Snapshot Quarantine Layer (SLSA L3 + in-toto + Sigstore)
- Selective Alignment Training Studio (DoRA / QLoRA DPO)

`impforge-cli upgrade` opens the Pro page.

## Why Rust

- Single binary, ~3 MB, zero runtime dependencies.
- Starts in < 5 ms — every command feels instant.
- Modules in `DeepSleep` by default → < 5 MB resident memory when idle.
- Memory-safe, audit-able, MIT-licensed, published to crates.io.

## Architecture — Neural Trias Lite

Every sub-crate is a **micro-program** registered with the `impforge-emergence` Orchestrator:

```
impforge-cli ─► Orchestrator
                 ├── impforge-core              (shared DTOs)
                 ├── impforge-scaffold          (template copier)
                 ├── impforge-models            (Ollama / HF / llama.cpp / Candle)
                 ├── impforge-mcp-server        (lazy MCP, 90 % token savings)
                 ├── impforge-contribute        (community wizard)
                 ├── impforge-export            (Ed25519-signed migration bundle)
                 └── impforge-autonomy          (doctor · self-update · watchdog · autopilot)
```

Every module implements `health()` / `capabilities()` / `power_mode()` / `self_heal()`.
The runtime tracks episodic memory in `~/.impforge-cli/memory.json` so every
decision can be audited after the fact.  This is the open-source distillation
of the Emergence Kernel pattern used in the commercial ImpForge Pro.

## Security model

- **Zero engine leak**: `impforge-cli` imports nothing from the commercial ImpForge codebase.
- **Migration pipeline**: `impforge-cli export-config` writes an **Ed25519-signed** JSON bundle that `impforge-aiimp` validates through its Quarantine Layer.  No network handshake.
- **Templates read-only**: bundled templates are hash-verified on every scaffold.
- **Prompt-injection scrubber**: every metadata field is sanitized before any LLM sees it.
- **Capability tokens**: cross-module dispatch uses NIST SP 800-207 scoped tokens.

Read the full security policy in [`docs/SECURITY.md`](docs/SECURITY.md).

## Contribute

Community contributions are welcomed and go through a zero-surprise local validation
pipeline BEFORE you open a PR:

```bash
impforge-cli contribute template     # interactive wizard
impforge-cli contribute skill        # same for skills
impforge-cli contribute mcp-manifest # same for MCP manifests
```

Your contribution is validated locally against the v1 schema, security-scanned,
and prompt-injection-scrubbed.  Only then does the tool open your browser at the
GitHub PR page with an auto-filled body.

## License

MIT · © ImpForge Maintainers. See [`LICENSE`](./LICENSE).

The commercial ImpForge Pro (engine, Pro Mesh, CJ rules, 4-Model Pipeline, Training
Studio) ships under **ELv2 + BUSL-1.1** and is NOT part of this repository.  It is
distributed exclusively via <https://impforge.com>.
