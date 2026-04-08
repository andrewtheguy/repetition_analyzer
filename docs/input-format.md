# Input Format

## Raw Input

The tool reads JSONL (JSON Lines) files -- one JSON object per line. Raw input can have any field names; the `preprocess` subcommand normalizes them to a canonical format.

## Canonical Format (after preprocessing)

After running `preprocess`, the output is a CSV file with a header row and these fixed columns:

```csv
id,text,start_ms,end_ms,start_formatted,end_formatted
019d...,Some transcript text,0,2500,00:00:00.000,00:00:02.500
```

All downstream commands (`analyze`, `enrich`, `extract-unique`) expect this format.

### Fields

| Field | Required | Description |
|---|---|---|
| `text` | yes | The text content to analyze |
| `id` | yes | Unique entry identifier (from `--id-key` or auto-generated UUIDv7) |
| `start_ms` | no | Start time in milliseconds |
| `end_ms` | no | End time in milliseconds |
| `start_formatted` | no | Human-readable start time (HH:MM:SS.mmm) |
| `end_formatted` | no | Human-readable end time (HH:MM:SS.mmm) |

### Timestamp Conversion

If only one timestamp form is available in the raw input, `preprocess` computes the other:

- `start_ms` present but no `start_formatted` → generates `"01:30:05.250"` from `5405250`
- `start_formatted` present but no `start_ms` → generates `5405250` from `"01:30:05.250"`

### ID Handling

- If `--id-key` is omitted: every entry gets a UUIDv7.
- If `--id-key` is set: every matching entry must have a non-null string or number value for that key. Missing or null values cause an error.

## Filtering

If your JSONL file contains mixed entry types, use `--filter` on the `preprocess` step to select only matching entries.

### String filtering (default)

```bash
--filter type=transcript
```

Matches entries where the `type` field equals the string `"transcript"`. This is equivalent to `--filter type:str=transcript`.

### Typed filtering

For non-string fields, specify the type with `key:type=value`:

| Syntax | Matches |
|---|---|
| `--filter key=value` | String field (default) |
| `--filter key:str=value` | String field (explicit) |
| `--filter key:bool=true` | Boolean field |
| `--filter key:int=1` | Integer field |
| `--filter key:float=0.5` | Float field |

**Behavior:**

- Lines that don't match the filter (or lack the filter key) are silently skipped.
- Null values for the filter key are silently skipped.
- If a field exists but has a different JSON type than specified (e.g., `--filter status:int=1` on a string `"1"`), the tool exits with a type mismatch error.

## Indexing

Valid entries are assigned sequential indices starting at 0, in file order. These indices appear throughout the analysis output and are used to cross-reference entries.

## Example

Raw input:

```jsonl
{"type": "stream_start"}
{"type": "transcript", "text": "Good morning everyone", "start_ms": 0, "end_ms": 2500, "start_formatted": "00:00:00.000", "end_formatted": "00:00:02.500"}
{"type": "metadata", "duration": 3600}
{"type": "transcript", "text": "Welcome to the broadcast", "start_ms": 2500, "end_ms": 5000, "start_formatted": "00:00:02.500", "end_formatted": "00:00:05.000"}
```

After `preprocess --filter type=transcript`:

```csv
id,text,start_ms,end_ms,start_formatted,end_formatted
019d...,Good morning everyone,0,2500,00:00:00.000,00:00:02.500
019d...,Welcome to the broadcast,2500,5000,00:00:02.500,00:00:05.000
```

The non-transcript lines are skipped, and the two entries receive indices 0 and 1.
