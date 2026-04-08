# Repetition Analyzer

A command-line tool for detecting repeated text in JSONL files. It reads any JSONL data and runs five complementary analyses to surface exact duplicates, near-duplicates, repeated phrases, repeated multi-entry blocks, and near-duplicate multi-entry blocks.

## Quick Start

```bash
cargo build --release

# Analyze any JSONL file (uses "text" field by default)
./target/release/repetition_analyzer analyze data.jsonl

# Filter to specific entry types
./target/release/repetition_analyzer analyze --filter type=transcript data.jsonl

# Use a custom text field
./target/release/repetition_analyzer analyze --text-key content data.jsonl

# JSON output
./target/release/repetition_analyzer analyze data.jsonl --format json > result.json

# Enrich result with timestamps from the original file
./target/release/repetition_analyzer enrich --source data.jsonl --result result.json > enriched.json

# Preprocess: filter entries and optionally insert UUIDs
./target/release/repetition_analyzer preprocess data.jsonl --filter type=transcript --new-id-key uuid_id > filtered.jsonl
```

## Subcommands

### `analyze`

Runs all analyses on a JSONL file and outputs a report.

| Option | Default | Description |
|---|---|---|
| `<file>` | -- | Path to the JSONL file |
| `--text-key` | `text` | JSON key containing the text to analyze |
| `--id-key` | -- | JSON key to use as entry ID (defaults to file line number) |
| `--filter` | -- | Filter entries by `key=value` or `key:type=value` (see [docs/input-format.md](docs/input-format.md)) |
| `--min-ngram` | 3 | Minimum word count for phrase detection |
| `--max-ngram` | 8 | Maximum word count for phrase detection |
| `--similarity-threshold` | 0.85 | Similarity ratio (0.0-1.0) for near-duplicate clustering |
| `--top-n` | 20 | Max results per section in human-readable output |
| `--min-count` | 3 | Minimum entry count for a phrase to be reported |
| `--min-seq-len` | 2 | Minimum entries in a repeated block |
| `--max-seq-len` | 8 | Maximum entries in a repeated block |
| `--min-seq-occurrences` | 2 | Minimum times a block must repeat |
| `--seq-similarity-threshold` | 0.80 | Similarity ratio for near-duplicate sequence matching |
| `--format` | `human` | Output format: `human` or `json` |

### `enrich`

Post-processes a JSON result file by joining it with the original JSONL source to inject timestamp data. This is a separate step so the core analysis stays generic.

| Option | Default | Description |
|---|---|---|
| `--source` | -- | Path to the original JSONL file |
| `--result` | -- | Path to the JSON result from `analyze` |
| `--start-key` | `start` | JSON key for start time (seconds) |
| `--end-key` | `end` | JSON key for end time (seconds) |
| `--start-formatted-key` | `start_formatted` | JSON key for formatted start time |
| `--end-formatted-key` | `end_formatted` | JSON key for formatted end time |
| `--text-key` | `text` | JSON key for text (must match what was used in `analyze`) |
| `--filter` | -- | Filter (must match what was used in `analyze`) |

### `preprocess`

Filters a JSONL file and optionally inserts a UUIDv7 column into each entry. Outputs filtered JSONL to stdout.

| Option | Default | Description |
|---|---|---|
| `<file>` | -- | Path to the JSONL file |
| `--text-key` | `text` | JSON key for text content (entries missing this field are skipped) |
| `--filter` | -- | Filter entries by `key=value` or `key:type=value` |
| `--new-id-key` | -- | If set, inserts a UUIDv7 into each entry under this key name |

See [docs/input-format.md](docs/input-format.md), [docs/analyses.md](docs/analyses.md), and [docs/output-format.md](docs/output-format.md) for details.

## Analyses

1. **Exact Duplicates** -- Groups entries with identical normalized text.
2. **Near-Duplicates** -- Clusters entries whose text is highly similar, catching minor variations.
3. **Repeated Phrases (N-grams)** -- Finds word sequences that recur across many entries.
4. **Repeated Sequences** -- Detects contiguous multi-entry blocks that repeat exactly.
5. **Near-Duplicate Sequences** -- Detects multi-entry blocks that repeat with minor text variations.

## Building

Requires a Rust toolchain. No external runtime dependencies.

```bash
cargo build --release
cargo test
```
