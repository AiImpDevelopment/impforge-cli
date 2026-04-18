# Security Policy

## Scope

`impforge-cli`, `impforge-aiimp-updater`, every crate under `crates/`, and
all bundled templates / skills / MCP manifests / compliance rules.

## Isolation contract

- `impforge-cli` **imports no proprietary ImpForge engine code**.  Every
  line in this repository is MIT-licensed and auditable.
- The Pro version (`impforge-aiimp`) is distributed **only as a Tauri
  binary** via <https://impforge.com>.  It does not share crates or
  build artefacts with this repository.
- Migration from CLI to Pro happens through an **Ed25519-signed JSON
  bundle** validated by the Pro app's Quarantine Layer.  There is no
  network handshake between the CLI and Pro app.

## Supported versions

We ship security fixes for the latest two minor releases on `main`.

## Reporting a vulnerability

Please email <security@impforge.com> with:

1. A reproduction of the issue (steps, exploit, or PoC).
2. The affected version (`impforge-cli --version`).
3. Your preferred disclosure timeline.

We aim to acknowledge within 24 h and ship a fix within 7 days for
high-severity issues, 30 days for medium, 90 days for low.

## Hardening guarantees

- `cargo audit` runs on every PR via CI.
- `cargo clippy --all-targets -- -D warnings` blocks merges.
- Every `#[tauri::command]`-style entry point runs through the
  `impforge_emergence::Orchestrator`'s capability discovery and is
  subject to the runtime's health / self-heal loop.
- No `unwrap()` in production Rust code.  Tests use `expect("why")`
  with a human-readable reason.
- Template scaffolding refuses absolute paths and `..` directory-escape
  sequences.
- Metadata entering any LLM prompt is wrapped as `TreatAsData` with
  unicode-directional-character stripping (OWASP LLM01:2025).

## What is NOT in scope

- Vulnerabilities in third-party MCP servers we reference.  Report those
  upstream.  Our manifests point at upstream repositories.
- Vulnerabilities in the user's local Ollama / llama.cpp / HuggingFace
  installations.
- Issues with the commercial `impforge-aiimp` app — report those via
  <https://impforge.com/support>.

## Public key pinning

- `impforge-aiimp-updater` pins an Ed25519 public key in
  `crates/impforge-aiimp-updater/src/pubkey.rs`.  Rotations are
  announced in [`CHANGELOG.md`](./CHANGELOG.md) and published on
  <https://impforge.com/security>.
