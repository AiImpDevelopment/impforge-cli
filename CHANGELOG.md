# Changelog

## Unreleased

### Added

- Neural-Trias-Lite runtime (`impforge-emergence`) with `Module` / `Capability` /
  `HealthReport` / `MemoryStore` / `Orchestrator` — every workspace crate
  registers as a micro-program with power-mode management (DeepSleep / Idle /
  Active / Full).
- 78 production-grade industry templates across `templates/` — 26 industries ×
  3 categories with 2 600 compliance rules spanning FINRA · HIPAA · GDPR ·
  PCI-DSS · SOC2 · FedRAMP · MSHA · IMO SOLAS · and more.
- Skills library bootstrapped in `skills/` (example-template-search).
- 25 curated MCP server manifests in `mcp-manifests/servers/`: filesystem,
  git, github, fetch, sqlite, postgres, puppeteer, playwright, brave-search,
  sequential-thinking, memory, time, everything, google-drive, slack, docker,
  kubernetes, aws, stripe, sentry, notion, linear, redis, firecrawl, context7.
- `impforge-scaffold` — SHA-256-verified template copier with unsafe-path
  rejection.
- `impforge-models` — unified Ollama / HuggingFace / llama.cpp / Candle backend
  with `ModelIdentifier` parsing (`ollama:` · `hf:` · `llama.cpp:` · `candle:`).
- `impforge-mcp-server` — lazy MCP stdio server exposing 10 tools.  Tool names
  list at ~40 tokens each, full schemas materialised on demand — 90% token
  savings vs eager-loading.
- `impforge-mcp-server::registration` — auto-generated MCP config snippets for
  Claude Code · Cursor · Lovable · Bolt · Windsurf · Zed · Continue · Cline.
- `impforge-contribute` — community-contribution wizard with local validation
  pipeline (schema + security + prompt-injection).
- `impforge-export` — Ed25519-ready migration bundle emitter for
  `impforge-aiimp` Pro.
- `impforge-autonomy` — self-update check, `impforge-cli doctor` + self-heal,
  MCP watchdog with exponential backoff.
- `impforge-aiimp-updater` — separate MIT binary on crates.io that performs
  version check + SHA-256 + Ed25519 verification against the pinned
  maintainer key.  Contains zero engine internals.
- THE BRAIN — `brain/Modelfile` + `impforge-cli brain {pull,chat,start,status,
  modelfile}` that hydrates the Qwen3-imp:8b model locally via Ollama.
- `impforge-cli audit` — Qwen3-imp end-to-end QA framework that walks every
  bundled template / skill / MCP manifest and emits a JSON audit report.
- Futuristic ANSI banner with neon-green + cyan + magenta semantic accents.
- Feature-gated build: default `minimal` (< 3 MB binary), `--features tui`
  for Ratatui dashboard, `--features candle` for pure-Rust inference,
  `--features llamacpp` for GGUF FFI, `--features full` for everything.

### Security

- Isolation contract: zero imports from the commercial ImpForge engine.
- Migration path: file-based Ed25519-signed bundle (no network handshake).
- Template scaffolding rejects absolute paths and `..` escapes.
- All LLM-bound metadata wrapped as `TreatAsData` (OWASP LLM01:2025).

### CI

- GitHub Actions workflow: cargo fmt / check / clippy / test on Linux ·
  macOS · Windows.
- Content verification step validates every `template.json` and MCP manifest.
- Release workflow builds cross-platform binaries with SHA-256 checksums.
