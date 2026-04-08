# Input Format

The tool reads JSONL (JSON Lines) files -- one JSON object per line. It works with any JSONL structure; you configure which fields to use via CLI options.

## Required Field

The only required field is the one containing text, specified by `--text-key` (defaults to `"text"`).

```json
{"text": "This is the text to analyze"}
```

Or with a custom key:

```json
{"content": "This is the text to analyze"}
```

Use `--text-key content` in that case.

## Optional: Entry ID

By default, each entry's ID is its 1-based line number in the file. You can specify a custom ID field with `--id-key`:

```json
{"text": "hello", "uid": "abc-123"}
```

Use `--id-key uid` to use `"abc-123"` as the ID instead of the line number.

## Optional: Filtering

If your JSONL file contains mixed entry types, use `--filter key=value` to select only matching entries:

```jsonl
{"type": "metadata", "info": "..."}
{"type": "transcription", "text": "Good morning everyone"}
{"type": "transcription", "text": "Welcome to the broadcast"}
```

Use `--filter type=transcription` to process only transcription entries. Lines that don't match the filter (or lack the text key) are silently skipped.

## Indexing

Valid entries are assigned sequential indices starting at 0, in file order. These indices appear throughout the analysis output and are used to cross-reference entries. Skipped lines (filtered out, missing text key, invalid JSON) do not consume an index.

## Optional: Timestamps (for enrichment)

Timestamp fields are not used during analysis. They are only relevant when using the `enrich` subcommand to post-process results. The default timestamp keys are `start`, `end`, `start_formatted`, and `end_formatted`, but these are configurable.

## Example

```jsonl
{"type": "transcription", "text": "Good morning everyone", "start": 0.0, "end": 2.5, "start_formatted": "00:00:00", "end_formatted": "00:00:02"}
{"type": "metadata", "duration": 3600}
{"type": "transcription", "text": "Welcome to the broadcast", "start": 2.5, "end": 5.0, "start_formatted": "00:00:02", "end_formatted": "00:00:05"}
{"type": "transcription", "text": "Good morning everyone", "start": 5.0, "end": 7.5, "start_formatted": "00:00:05", "end_formatted": "00:00:07"}
```

With `--filter type=transcription`, the metadata line is skipped, and the three transcription entries receive indices 0, 1, and 2.
