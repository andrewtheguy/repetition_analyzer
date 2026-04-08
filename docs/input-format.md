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

If your JSONL file contains mixed entry types, use `--filter` to select only matching entries.

### String filtering (default)

```bash
--filter type=transcription
```

Matches entries where the `type` field equals the string `"transcription"`. This is equivalent to `--filter type:str=transcription`.

### Typed filtering

For non-string fields, specify the type with `key:type=value`:

| Syntax | Matches |
|---|---|
| `--filter key=value` | String field (default) |
| `--filter key:str=value` | String field (explicit) |
| `--filter key:bool=true` | Boolean field |
| `--filter key:int=1` | Integer field |
| `--filter key:float=0.5` | Float field |

```jsonl
{"type": "metadata", "info": "..."}
{"type": "transcription", "text": "Good morning everyone"}
{"type": "transcription", "text": "Welcome to the broadcast"}
```

Use `--filter type=transcription` to process only transcription entries.

**Behavior:**

- Lines that don't match the filter (or lack the filter key) are silently skipped.
- Null values for the filter key are silently skipped.
- If a field exists but has a different JSON type than specified (e.g., `--filter status:int=1` on a string `"1"`), the tool exits with a type mismatch error.

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
