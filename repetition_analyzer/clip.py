"""Clip a preprocessed CSV to a time range."""

import csv
import sys
from typing import Any

from .preprocess import formatted_to_ms


def _parse_time_to_ms(value: str) -> int:
    """Parse a time value as either milliseconds or HH:MM:SS.mmm."""
    if value.isdigit():
        return int(value)
    ms = formatted_to_ms(value)
    if ms is None:
        raise ValueError(f"invalid time format: {value!r} (expected HH:MM:SS.mmm or milliseconds)")
    return ms


def run_clip(config: dict[str, Any]) -> None:
    after_ms = _parse_time_to_ms(config["after"]) if config["after"] else None
    before_ms = _parse_time_to_ms(config["before"]) if config["before"] else None

    if after_ms is None and before_ms is None:
        print("error: at least one of --after or --before is required", file=sys.stderr)
        sys.exit(1)

    kept = 0
    dropped = 0
    with open(config["file"], newline="") as f:
        reader = csv.DictReader(f)
        assert reader.fieldnames is not None
        writer = csv.DictWriter(sys.stdout, fieldnames=reader.fieldnames)
        writer.writeheader()
        for row in reader:
            start = int(row["start_ms"])
            if before_ms is not None and start < before_ms:
                dropped += 1
                continue
            if after_ms is not None and start >= after_ms:
                dropped += 1
                continue
            writer.writerow(row)
            kept += 1

    print(f"Kept {kept}, dropped {dropped}", file=sys.stderr)
