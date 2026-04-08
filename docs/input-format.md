# Input Format

The tool reads JSONL (JSON Lines) files -- one JSON object per line.

## Entry Schema

```json
{
  "type": "transcription",
  "start": 12.5,
  "end": 15.0,
  "start_formatted": "00:00:12",
  "end_formatted": "00:00:15",
  "text": "Welcome back to the show"
}
```

### Fields

| Field | Type | Required | Description |
|---|---|---|---|
| `type` | string | yes | Must be `"transcription"` to be processed |
| `start` | number | yes | Start time in seconds from the beginning of the broadcast |
| `end` | number | yes | End time in seconds |
| `start_formatted` | string | yes | Human-readable start timestamp (e.g., `"01:23:45"`) |
| `end_formatted` | string | yes | Human-readable end timestamp |
| `text` | string | yes | The transcribed text content |

## Filtering Rules

- Only entries where `type` equals `"transcription"` are kept. All other types (metadata, silence markers, etc.) are skipped.
- If any of the required fields are missing from a transcription entry, that entry is skipped.
- Entries are processed in file order. Each valid entry receives a sequential index starting at 0. This ordering matters for sequence analysis and for interpreting `indices` in the output.

## Example

```jsonl
{"type": "transcription", "start": 0.0, "start_formatted": "00:00:00", "text": "Good morning everyone", "end": 2.5, "end_formatted": "00:00:02"}
{"type": "metadata", "duration": 3600}
{"type": "transcription", "start": 2.5, "start_formatted": "00:00:02", "text": "Welcome to the broadcast", "end": 5.0, "end_formatted": "00:00:05"}
{"type": "transcription", "start": 5.0, "start_formatted": "00:00:05", "text": "Good morning everyone", "end": 7.5, "end_formatted": "00:00:07"}
```

In this example, the metadata line is skipped, and the three transcription entries receive indices 0, 1, and 2 respectively.
