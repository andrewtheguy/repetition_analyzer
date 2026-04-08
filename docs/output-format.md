# Output Format

The tool supports two output formats selected via `--format`.

Progress messages (parsing, timing) are always written to stderr, so they don't interfere with piping stdout.

## Human-Readable (`--format human`)

This is the default. The report contains five sections:

1. **Exact Duplicates** -- Each group shows the repetition count, the text (truncated if long), and the timestamps of the first and last occurrence.
2. **Near-Duplicate Clusters** -- Each cluster shows a representative sample and up to five variant texts.
3. **Most Repeated Phrases** -- N-grams sorted by frequency, showing word count and entry count.
4. **Repeated Segment Blocks** -- Each block shows how many entries it spans, how many times it repeats, the duration, the text of each entry, and occurrence timestamps.
5. **Summary** -- Aggregate statistics including total duplicate groups, clusters, the most repeated phrase and block, and an estimate of how much broadcast time is consumed by exact duplicates.

The `--top-n` option limits how many results appear per section.

## JSON (`--format json`)

Produces a single JSON object on stdout. The structure:

```json
{
  "file_path": "string",
  "total_entries": 0,
  "total_duration_secs": 0.0,
  "exact_duplicates": [ ... ],
  "near_duplicates": [ ... ],
  "ngrams": [ ... ],
  "repeated_sequences": [ ... ]
}
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
  "entry_indices": [1, 8, 15, 22, ...]
}
```

### `repeated_sequences`

```json
{
  "length": 3,
  "occurrences": [
    { "start_index": 10, "start_time": "00:05:30" },
    { "start_index": 85, "start_time": "00:42:10" }
  ],
  "entry_texts": [
    "first entry text",
    "second entry text",
    "third entry text"
  ],
  "duration_secs": 15.0
}
```

### Notes

- JSON output includes **all** results (not limited by `--top-n`), so downstream consumers can apply their own filtering.
- All entry indices are zero-based and correspond to the sequential index assigned during parsing (see [input-format.md](input-format.md)).
- Timestamps in `start_time` are the `start_formatted` values from the original input data.
