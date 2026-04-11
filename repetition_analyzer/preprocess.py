"""Preprocess a JSONL file into canonical CSV format."""

import csv
import json
import sys
import uuid
from typing import Any


def ms_to_formatted(ms: int) -> str:
    total_secs = ms // 1000
    millis = ms % 1000
    hours = total_secs // 3600
    minutes = (total_secs % 3600) // 60
    seconds = total_secs % 60
    return f"{hours:02}:{minutes:02}:{seconds:02}.{millis:03}"


def formatted_to_ms(s: str) -> int | None:
    parts = s.split(".")
    if len(parts) != 2:
        return None
    hms = parts[0]
    millis_str = parts[1]
    hms_parts = hms.split(":")
    if len(hms_parts) != 3:
        return None
    # Normalize fractional seconds to exactly 3 digits (milliseconds)
    millis_str = millis_str[:3].ljust(3, "0")
    try:
        hours = int(hms_parts[0])
        minutes = int(hms_parts[1])
        seconds = int(hms_parts[2])
        millis = int(millis_str)
    except ValueError:
        return None
    return (hours * 3600 + minutes * 60 + seconds) * 1000 + millis


# -- Filter parsing --

FILTER_TYPES = {"str", "bool", "int", "float"}


def parse_filter(filter_str: str | None) -> dict[str, str] | None:
    if not filter_str:
        return None
    key_part, _, value = filter_str.partition("=")
    if not _:
        raise ValueError("--filter must be in key=value or key:type=value format")
    if ":" in key_part:
        key, filter_type = key_part.split(":", 1)
        if filter_type not in FILTER_TYPES:
            raise ValueError(f"unknown filter type '{filter_type}', expected str, bool, int, or float")
    else:
        key = key_part
        filter_type = "str"
    return {"key": key, "value": value, "type": filter_type}


def filter_matches(json_val: Any, parsed_filter: dict[str, str]) -> bool | None:
    """Check if a JSON value matches a filter. Returns True/False or raises on type mismatch."""
    if json_val is None:
        return False
    ft = parsed_filter["type"]
    val = parsed_filter["value"]
    key = parsed_filter["key"]

    if ft == "str":
        if not isinstance(json_val, str):
            raise TypeError(f"filter type mismatch for key '{key}': expected string, got {type(json_val).__name__}")
        return json_val == val
    elif ft == "bool":
        expected = val.lower() == "true"
        if not isinstance(json_val, bool):
            raise TypeError(f"filter type mismatch for key '{key}': expected bool, got {type(json_val).__name__}")
        return json_val == expected
    elif ft == "int":
        expected = int(val)
        if not isinstance(json_val, int) or isinstance(json_val, bool):
            raise TypeError(f"filter type mismatch for key '{key}': expected number, got {type(json_val).__name__}")
        return json_val == expected
    elif ft == "float":
        expected = float(val)
        if not isinstance(json_val, (int, float)) or isinstance(json_val, bool):
            raise TypeError(f"filter type mismatch for key '{key}': expected number, got {type(json_val).__name__}")
        return json_val == expected
    return False


def truncate_hallucinated_repeats(text: str, min_repeats: int = 10, max_pattern_len: int = 30) -> str:
    """Truncate consecutively repeating patterns caused by speech-to-text hallucination."""
    n = len(text)
    if n < 100:
        return text
    for pat_len in range(2, min(max_pattern_len + 1, n // min_repeats + 1)):
        for start in range(n - pat_len * min_repeats + 1):
            pattern = text[start : start + pat_len]
            pos = start + pat_len
            count = 1
            while pos + pat_len <= n and text[pos : pos + pat_len] == pattern:
                count += 1
                pos += pat_len
            if count >= min_repeats:
                return text[: start + pat_len] + "(indistinguishable speech)"
    return text


def process_entry(obj: dict[str, Any], config: dict[str, Any], parsed_filter: dict[str, str] | None) -> dict[str, str] | None:
    """Process a single JSONL entry. Returns canonical row dict or None to skip."""
    # Apply filter
    if parsed_filter:
        key = parsed_filter["key"]
        if key not in obj:
            return None
        result = filter_matches(obj[key], parsed_filter)
        if not result:
            return None

    # Skip entries missing or empty text
    text = obj.get(config["text_key"])
    if not text or not isinstance(text, str) or not text.strip():
        return None
    text = truncate_hallucinated_repeats(text)

    # ID
    id_key = config.get("id_key")
    if id_key:
        id_val = obj.get(id_key)
        if id_val is None:
            raise ValueError(f"missing or null id key '{id_key}'")
        entry_id = str(id_val)
    else:
        entry_id = str(uuid.uuid4())

    # Timestamps
    start_ms_val = obj.get(config["start_ms_key"])
    end_ms_val = obj.get(config["end_ms_key"])
    start_fmt_val = obj.get(config["start_formatted_key"])
    end_fmt_val = obj.get(config["end_formatted_key"])

    if isinstance(start_ms_val, (int, float)):
        start_ms = str(int(start_ms_val))
        start_formatted = start_fmt_val if isinstance(start_fmt_val, str) else ms_to_formatted(int(start_ms_val))
    elif isinstance(start_fmt_val, str):
        ms = formatted_to_ms(start_fmt_val)
        if ms is None:
            raise ValueError(f"invalid start timestamp format: '{start_fmt_val}'")
        start_ms = str(ms)
        start_formatted = start_fmt_val
    else:
        raise ValueError(f"missing start timestamp (expected '{config['start_ms_key']}' or '{config['start_formatted_key']}')")

    if isinstance(end_ms_val, (int, float)):
        end_ms = str(int(end_ms_val))
        end_formatted = end_fmt_val if isinstance(end_fmt_val, str) else ms_to_formatted(int(end_ms_val))
    elif isinstance(end_fmt_val, str):
        ms = formatted_to_ms(end_fmt_val)
        if ms is None:
            raise ValueError(f"invalid end timestamp format: '{end_fmt_val}'")
        end_ms = str(ms)
        end_formatted = end_fmt_val
    else:
        raise ValueError(f"missing end timestamp (expected '{config['end_ms_key']}' or '{config['end_formatted_key']}')")

    return {
        "id": entry_id,
        "text": text,
        "start_ms": start_ms,
        "end_ms": end_ms,
        "start_formatted": start_formatted,
        "end_formatted": end_formatted,
    }


def run_preprocess(config: dict[str, Any]) -> None:
    parsed_filter = parse_filter(config.get("filter"))
    writer = csv.writer(sys.stdout)
    writer.writerow(["id", "text", "start_ms", "end_ms", "start_formatted", "end_formatted"])
    count = 0
    prev_id: str | None = None

    with open(config["file"]) as f:
        for line_num, line in enumerate(f):
            line = line.strip()
            if not line:
                continue
            try:
                obj = json.loads(line)
            except json.JSONDecodeError as e:
                raise ValueError(f"line {line_num + 1}: invalid JSON: {e}") from e

            try:
                row = process_entry(obj, config, parsed_filter)
            except (ValueError, TypeError) as e:
                raise ValueError(f"line {line_num + 1}: {e}") from e

            if row is None:
                continue

            # Check ascending IDs
            if prev_id is not None:
                try:
                    is_ascending = int(prev_id) < int(row["id"])
                except ValueError:
                    is_ascending = prev_id < row["id"]
                if not is_ascending:
                    raise ValueError(
                        f"line {line_num + 1}: id '{row['id']}' is not ascending from previous id '{prev_id}'"
                    )

            prev_id = row["id"]
            writer.writerow([row["id"], row["text"], row["start_ms"], row["end_ms"], row["start_formatted"], row["end_formatted"]])
            count += 1

    print(f"Wrote {count} entries", file=sys.stderr)
