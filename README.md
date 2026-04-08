# Repetition Analyzer

A command-line tool for detecting repeated text in broadcast transcriptions. It reads timestamped transcription data in JSONL format and runs four complementary analyses to surface exact duplicates, near-duplicates, repeated phrases, and repeated multi-entry blocks.

## Quick Start

```bash
cargo build --release
./target/release/repetition_analyzer /path/to/transcription.jsonl
```

Or run directly from source:

```bash
cargo run -- /path/to/transcription.jsonl
```

## Input Format

The tool reads JSONL files where each line is a JSON object. Only entries with `"type": "transcription"` are processed; other entry types are silently skipped. Each transcription entry must include `start`, `end`, `start_formatted`, `end_formatted`, and `text` fields.

See [docs/input-format.md](docs/input-format.md) for the full schema and examples.

## Analyses

1. **Exact Duplicates** -- Groups entries with identical normalized text.
2. **Near-Duplicates** -- Clusters entries whose text is highly similar (configurable threshold), catching minor variations.
3. **Repeated Phrases (N-grams)** -- Finds word sequences that recur across many entries, with deduplication so longer phrases suppress redundant sub-phrases.
4. **Repeated Sequences** -- Detects contiguous multi-entry blocks that appear more than once in the broadcast.
5. **Near-Duplicate Sequences** -- Detects multi-entry blocks that repeat with minor text variations (e.g., recurring news/traffic reports where transcription differs slightly between airings).

See [docs/analyses.md](docs/analyses.md) for how each analysis works.

## Output

By default the tool prints a human-readable text report to stdout. Pass `--format json` for structured JSON output suitable for programmatic consumption.

See [docs/output-format.md](docs/output-format.md) for details on both formats.

## CLI Options

| Option | Default | Description |
|---|---|---|
| `<file>` | -- | Path to the JSONL transcription file |
| `--min-ngram` | 3 | Minimum word count for phrase detection |
| `--max-ngram` | 8 | Maximum word count for phrase detection |
| `--similarity-threshold` | 0.85 | Similarity ratio (0.0-1.0) for near-duplicate clustering |
| `--top-n` | 20 | Max results per section in human-readable output |
| `--min-count` | 3 | Minimum entry count for a phrase to be reported |
| `--min-seq-len` | 2 | Minimum entries in a repeated block |
| `--max-seq-len` | 8 | Maximum entries in a repeated block |
| `--min-seq-occurrences` | 2 | Minimum times a block must repeat |
| `--seq-similarity-threshold` | 0.80 | Similarity ratio for near-duplicate sequence matching |
| `--format` | human | Output format: `human` or `json` |

## Building

Requires a Rust toolchain. No external runtime dependencies.

```bash
cargo build --release
cargo test
```
