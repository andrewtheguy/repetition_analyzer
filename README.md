# Repetition Analyzer

A command-line tool for detecting repeated text in JSONL files. It reads any JSONL data and runs five complementary analyses to surface exact duplicates, near-duplicates, repeated phrases, repeated multi-entry blocks, and near-duplicate multi-entry blocks.

Python CLI with performance-critical algorithms (Levenshtein distance, n-gram extraction, sequence detection) implemented in Rust via PyO3.

## Quick Start

```bash
# Install (requires Rust toolchain + Python 3.10+)
uv sync --group dev
maturin develop --manifest-path native-helper/Cargo.toml --features python --skip-install

# Preprocess: filter, normalize field names, and assign unique IDs
uv run repetition-analyzer preprocess data.jsonl --filter type=transcript > filtered.csv

# Analyze the preprocessed file
uv run repetition-analyzer analyze filtered.csv

# JSON output
uv run repetition-analyzer analyze filtered.csv --format json > result.json

# Enrich result with timestamps from the preprocessed file
uv run repetition-analyzer enrich --source filtered.csv --result result.json > enriched.json

# Extract unique/repeated segments
uv run repetition-analyzer extract-unique --source filtered.csv --result result.json > segments.json

# HTML visualization
uv run repetition-analyzer plot enriched.json
```

## Subcommands

### `preprocess`

Filters a JSONL file, normalizes field names to canonical keys, and ensures every entry has a unique `id`. If only millisecond or formatted timestamps are available, the missing form is computed automatically.

| Option | Default | Description |
|---|---|---|
| `<file>` | -- | Path to the JSONL file |
| `--text-key` | `text` | Input JSON key for text content |
| `--id-key` | -- | Input JSON key for existing unique ID (omit to auto-generate) |
| `--start-ms-key` | `start_ms` | Input JSON key for start time in milliseconds |
| `--end-ms-key` | `end_ms` | Input JSON key for end time in milliseconds |
| `--start-formatted-key` | `start_formatted` | Input JSON key for formatted start time (HH:MM:SS.mmm) |
| `--end-formatted-key` | `end_formatted` | Input JSON key for formatted end time (HH:MM:SS.mmm) |
| `--filter` | -- | Filter entries by `key=value` or `key:type=value` (see [docs/input-format.md](docs/input-format.md)) |

### `analyze`

Runs all analyses on a preprocessed CSV file and outputs a report.

| Option | Default | Description |
|---|---|---|
| `<file>` | -- | Path to the preprocessed CSV file |
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

Post-processes a JSON result file by joining it with the preprocessed CSV source to inject timestamp data.

### `extract-unique`

Segments entries into contiguous unique/repeated ranges based on all repetition analyses.

### `extract-segments`

Extracts segments to markdown/text files, optionally classifying repeated segments by station.

### `plot`

Generates an interactive HTML bar chart from enriched JSON.

## Analyses

1. **Exact Duplicates** -- Groups entries with identical normalized text.
2. **Near-Duplicates** -- Clusters entries whose text is highly similar, catching minor variations.
3. **Repeated Phrases (N-grams)** -- Finds word sequences that recur across many entries.
4. **Repeated Sequences** -- Detects contiguous multi-entry blocks that repeat exactly.
5. **Near-Duplicate Sequences** -- Detects multi-entry blocks that repeat with minor text variations.

## Development

Requires a Rust toolchain and Python 3.10+.

```bash
# Setup
uv sync --group dev
maturin develop --manifest-path native-helper/Cargo.toml --features python --skip-install

# Run tests
cd native-helper && cargo test && cargo clippy  # Rust algorithm tests
uv run pytest                                    # Python tests
```

See [docs/input-format.md](docs/input-format.md), [docs/analyses.md](docs/analyses.md), and [docs/output-format.md](docs/output-format.md) for details.
