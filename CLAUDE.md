# Repetition Analyzer

Rust CLI tool that analyzes repeated text in JSONL broadcast transcription files.

## Build & Run

```
cargo build --release
cargo run --release -- /path/to/file.jsonl
cargo test
cargo clippy
```

## Architecture

- `src/main.rs` — CLI args (clap) + orchestration
- `src/parse.rs` — JSONL parsing, `Transcription` struct
- `src/similarity.rs` — text normalization, bounded Levenshtein distance
- `src/exact.rs` — exact duplicate + near-duplicate clustering
- `src/ngrams.rs` — n-gram extraction with longest-phrase-wins dedup
- `src/sequences.rs` — repeated multi-entry block detection via fingerprinting
- `src/report.rs` — formatted stdout report

## Conventions

- No backward compatibility shims. Delete unused code outright — no `_unused` renames, no re-exports, no `// removed` comments.
- No unnecessary abstractions. Inline simple logic rather than extracting helpers for one-time use.
- Keep dependencies minimal. Prefer inline implementations over adding crates for trivial functionality.
- All code must pass `cargo clippy` with no warnings.
