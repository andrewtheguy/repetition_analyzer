# Output Format

The tool supports two output formats selected via `--format` on the `analyze` subcommand.

Progress messages (parsing, timing) are always written to stderr, so they don't interfere with piping stdout.

## Human-Readable (`--format human`)

This is the default. The report contains six sections:

1. **Exact Duplicates** -- Each group shows the repetition count, the text (truncated if long), and the IDs of the first and last occurrence.
2. **Near-Duplicate Clusters** -- Each cluster shows a representative sample and up to five variant texts.
3. **Most Repeated Phrases** -- N-grams sorted by frequency, showing word count and entry count.
4. **Repeated Segment Blocks** -- Each block shows how many entries it spans, how many times it repeats, the text of each entry, and occurrence start indices.
5. **Near-Duplicate Segment Blocks** -- Each pattern shows occurrence count, block length, average similarity, and up to three occurrences with full entry texts so variations are visible side-by-side.
6. **Summary** -- Aggregate statistics including total duplicate groups, clusters, and near-duplicate sequence patterns.

The `--top-n` option limits how many results appear per section.

## JSON (`--format json`)

Produces a single JSON object on stdout:

```json
{
  "file_path": "string",
  "total_entries": 0,
  "id_column": null,
  "id_from_line_number": true,
  "exact_duplicates": [ ... ],
  "near_duplicates": [ ... ],
  "ngrams": [ ... ],
  "repeated_sequences": [ ... ],
  "near_duplicate_sequences": [ ... ]
}
```

- `id_column`: The `--id-key` value if provided (e.g., `"uid"`), or `null` when using line numbers.
- `id_from_line_number`: `true` when IDs are auto-generated from file line numbers, `false` when sourced from a JSONL column.
```

### `exact_duplicates`

```json
{
  "canonical_text": "the repeated text",
  "count": 5,
  "indices": [0, 12, 45, 78, 102]
}
```

### `near_duplicates`

```json
{
  "representative_text": "sample text from the cluster",
  "members": [
    [3, "sample text from the cluster"],
    [17, "sample text from a cluster"]
  ],
  "total_count": 2
}
```

Each member is a tuple of `[entry_index, text]`.

### `ngrams`

```json
{
  "ngram": "welcome back to the show",
  "n": 5,
  "count": 12,
  "entry_indices": [1, 8, 15, 22]
}
```

### `repeated_sequences`

```json
{
  "length": 3,
  "occurrences": [
    { "start_index": 10 },
    { "start_index": 85 }
  ],
  "entry_texts": [
    "first entry text",
    "second entry text",
    "third entry text"
  ]
}
```

### `near_duplicate_sequences`

```json
{
  "length": 2,
  "occurrences": [
    {
      "start_index": 150,
      "entry_texts": [
        "Call Pacific Coast Termite today at 800 Pacific...",
        "For a free home inspection..."
      ]
    },
    {
      "start_index": 320,
      "entry_texts": [
        "Cau Pacific Coast termite today at 800 Pacific...",
        "For a free home inspection..."
      ]
    }
  ],
  "representative_texts": [
    "Call Pacific Coast Termite today at 800 Pacific...",
    "For a free home inspection..."
  ],
  "avg_similarity": 0.88
}
```

Each occurrence stores its own `entry_texts` since the texts differ between occurrences.

### Notes

- JSON output includes **all** results (not limited by `--top-n`), so downstream consumers can apply their own filtering.
- All entry indices are zero-based and correspond to the sequential index assigned during parsing (see [input-format.md](input-format.md)).
- The base `analyze` output contains no timestamp or duration data. Use the `enrich` subcommand to add that.

## Enriched Output (`enrich` subcommand)

Running `enrich` on a result JSON file adds timestamp data from the original JSONL source:

- **Top level:** `total_duration_secs` is added (last entry end - first entry start).
- **`start_index` objects:** Each object with a `start_index` field gets `start`, `end`, `start_formatted`, `end_formatted` injected (if available in the source).
- **`indices` arrays:** An `index_timestamps` array is added alongside, with each element containing the index plus its timestamp fields.

```bash
repetition_analyzer enrich --source data.jsonl --result result.json > enriched.json
```
