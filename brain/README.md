# THE BRAIN — ImpForge's 8B Production Intelligence

<p align="center">
  <b>The exact 8B AI model that powers ImpForge Pro — run it locally, offline, forever.</b>
</p>

## One-line install via impforge-cli

```bash
impforge-cli brain pull
```

## Manual install via Ollama

```bash
ollama pull impforge/qwen3-imp-brain-8b:latest
ollama create brain -f brain/Modelfile
ollama run brain
```

## What it is

- **Base model**: Qwen3-8B (Apache-2.0)
- **Fine-tune**: `impforge/qwen3-imp-brain-8b` on HuggingFace (Apache-2.0)
- **System prompt**: Josiefied (uncensored helpfulness) + ImpForge Safety alignment
- **Size**: ~5 GB (Q4_K_M quantisation)
- **Context**: 32 K tokens native, up to 128 K via YaRN rope-scaling
- **Modes**: Chat (default) + deep-reasoning (enable via Qwen3 `<think>` tag)

## Why it matters

This is **the same intelligence core** that ImpForge Pro uses for code generation,
template orchestration, and user-intent classification.  In the Pro version, it
runs inside a **4-model pipeline** (SmolLM2 classifier → BRAIN → Qwen2.5-Coder
fast-hands → nomic-embed-text memory).

In `impforge-cli`, **you get THE BRAIN alone**, Apache-2.0, offline, forever.
The 4-model pipeline orchestration + Pro Mesh + 157 870 quality rules
**stay Pro-only** — see [`../README.md`](../README.md) for the feature matrix.

## Privacy

- **100 % local** after pull.  Ollama binds to 127.0.0.1 only.
- **No telemetry** — ImpForge does not see your prompts.
- **No cloud fallback** — if you're offline, BRAIN still answers.

## Performance

On typical developer hardware:

| GPU | Tokens/sec | Startup |
|-----|-----------|---------|
| RTX 4090 24 GB   | 85-95 tok/s  | 3 s  |
| RTX 4070 12 GB   | 60-70 tok/s  | 4 s  |
| RX 7800 XT 16 GB | 50-60 tok/s  | 5 s  |
| Apple M2 Pro 16 GB | 35-45 tok/s | 6 s |
| Apple M1 Air 8 GB  | 15-20 tok/s | 10 s |
| CPU only (32 GB RAM) | 8-12 tok/s  | 20 s |

## License

Apache-2.0 for the model weights.  MIT for the Modelfile + this README.
The `impforge-aiimp` commercial app and its 4-model orchestration pipeline
remain Elastic-2.0 + BUSL-1.1 and are NOT part of this repository.
