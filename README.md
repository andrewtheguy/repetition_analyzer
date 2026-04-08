# Repetition Analyzer

A command-line tool for detecting repeated text in JSONL files. It reads any JSONL data and runs five complementary analyses to surface exact duplicates, near-duplicates, repeated phrases, repeated multi-entry blocks, and near-duplicate multi-entry blocks.

## Quick Start

```bash
cargo build --release

# Preprocess: filter, normalize field names, and assign unique IDs
./target/release/repetition_analyzer preprocess data.jsonl --filter type=transcript > filtered.jsonl

# Analyze the preprocessed file
./target/release/repetition_analyzer analyze filtered.jsonl

# JSON output
./target/release/repetition_analyzer analyze filtered.jsonl --format json > result.json

# Enrich result with timestamps from the preprocessed file
./target/release/repetition_analyzer enrich --source filtered.jsonl --result result.json > enriched.json

# Extract unique near-duplicate cluster representatives
./target/release/repetition_analyzer extract-unique --source filtered.jsonl --result result.json > unique.json
```

## Workflow

All data flows through `preprocess` first, which produces a canonical JSONL format:

```json
{"text": "...", "id": "uuid-or-existing", "start_ms": 0, "end_ms": 2500, "start_formatted": "00:00:00.000", "end_formatted": "00:00:02.500"}
```

Downstream commands (`analyze`, `enrich`, `extract-unique`) expect this format with no additional field-mapping arguments.

## Subcommands

### `preprocess`

Filters a JSONL file, normalizes field names to canonical keys, and ensures every entry has a unique `id`. If only millisecond or formatted timestamps are available, the missing form is computed automatically.

| Option | Default | Description |
|---|---|---|
| `<file>` | -- | Path to the JSONL file |
| `--text-key` | `text` | Input JSON key for text content |
| `--id-key` | -- | Input JSON key for existing unique ID (omit to auto-generate UUIDv7) |
| `--start-ms-key` | `start_ms` | Input JSON key for start time in milliseconds |
| `--end-ms-key` | `end_ms` | Input JSON key for end time in milliseconds |
| `--start-formatted-key` | `start_formatted` | Input JSON key for formatted start time (HH:MM:SS.mmm) |
| `--end-formatted-key` | `end_formatted` | Input JSON key for formatted end time (HH:MM:SS.mmm) |
| `--filter` | -- | Filter entries by `key=value` or `key:type=value` (see [docs/input-format.md](docs/input-format.md)) |

### `analyze`

Runs all analyses on a preprocessed JSONL file and outputs a report.

| Option | Default | Description |
|---|---|---|
| `<file>` | -- | Path to the preprocessed JSONL file |
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

Post-processes a JSON result file by joining it with the preprocessed JSONL source to inject timestamp data.

| Option | Default | Description |
|---|---|---|
| `--source` | -- | Path to the preprocessed JSONL file |
| `--result` | -- | Path to the JSON result from `analyze` |

### `extract-unique` (Experimental)

Extracts one representative per near-duplicate cluster (the last occurrence by index) with timestamp metadata.

| Option | Default | Description |
|---|---|---|
| `--source` | -- | Path to the preprocessed JSONL file |
| `--result` | -- | Path to the JSON result from `analyze` |

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
