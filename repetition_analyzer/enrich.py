"""Timestamp enrichment and unique/repeated segmentation."""

import csv
import json
import sys
from typing import Any


def _non_empty(s: str) -> str | None:
    return s if s else None


def build_entry_lookup(source: str) -> list[dict[str, Any]]:
    """Build lookup from a preprocessed CSV file."""
    entries = []
    with open(source, newline="") as f:
        reader = csv.DictReader(f)
        for row in reader:
            entries.append({
                "id": _non_empty(row.get("id", "")),
                "text": _non_empty(row.get("text", "")),
                "start_ms": int(row["start_ms"]) if row.get("start_ms") else None,
                "end_ms": int(row["end_ms"]) if row.get("end_ms") else None,
                "start_formatted": _non_empty(row.get("start_formatted", "")),
                "end_formatted": _non_empty(row.get("end_formatted", "")),
            })
    return entries


def _inject_entry_info(target: dict[str, Any], info: dict[str, Any]) -> None:
    for key in ("start_ms", "end_ms", "start_formatted", "end_formatted", "id", "text"):
        if info.get(key) is not None:
            target[key] = info[key]


def enrich_value(value: Any, lookup: list[dict[str, Any]]) -> None:
    """Recursively enrich JSON value with timestamp data from lookup."""
    if isinstance(value, dict):
        # If this object has a start_index, inject timestamp and id fields
        start_index = value.get("start_index")
        if isinstance(start_index, int) and 0 <= start_index < len(lookup):
            _inject_entry_info(value, lookup[start_index])

        # For arrays whose elements start with [index, ...], inject timestamps
        for key in ("indices", "members", "entry_indices"):
            if key in value and isinstance(value[key], list):
                ts_array = []
                for item in value[key]:
                    if isinstance(item, list) and item and isinstance(item[0], int):
                        idx = item[0]
                        ts = {"index": idx}
                        if 0 <= idx < len(lookup):
                            _inject_entry_info(ts, lookup[idx])
                        ts_array.append(ts)
                value["index_timestamps"] = ts_array

        # Recurse into all values
        for v in value.values():
            enrich_value(v, lookup)

    elif isinstance(value, list):
        for v in value:
            enrich_value(v, lookup)


def collect_repeated_indices(result_json: dict[str, Any]) -> set[int]:
    """Collect all entry indices covered by any repetition pattern."""
    repeated: set[int] = set()

    # From exact_duplicates: indices are [[index, id], ...]
    for group in result_json.get("exact_duplicates", []):
        for entry in group.get("indices", []):
            if isinstance(entry, list) and entry:
                repeated.add(entry[0])

    # From near_duplicates: members are [[index, id, text], ...]
    for cluster in result_json.get("near_duplicates", []):
        for member in cluster.get("members", []):
            if isinstance(member, list) and member:
                repeated.add(member[0])

    # From repeated_sequences
    for seq in result_json.get("repeated_sequences", []):
        length = seq.get("length", 0)
        for occ in seq.get("occurrences", []):
            start = occ.get("start_index", 0)
            for offset in range(length):
                repeated.add(start + offset)

    # From near_duplicate_sequences
    for seq in result_json.get("near_duplicate_sequences", []):
        length = seq.get("length", 0)
        for occ in seq.get("occurrences", []):
            start = occ.get("start_index", 0)
            for offset in range(length):
                repeated.add(start + offset)

    return repeated


def _consolidate_repeated(
    repeated: set[int], max_unique_gap: int, min_repeated_island: int,
) -> set[int]:
    """Consolidate repeated indices to reduce fragmentation.

    Pass 1 (close): fill unique gaps of <= max_unique_gap between repeated indices.
    Pass 2 (open): remove contiguous repeated runs of <= min_repeated_island entries.
    """
    if not repeated:
        return repeated

    result = set(repeated)

    # Pass 1: close gaps between nearby repeated indices.
    sorted_rep = sorted(result)
    for a, b in zip(sorted_rep, sorted_rep[1:]):
        gap = b - a - 1
        if 0 < gap <= max_unique_gap:
            result.update(range(a + 1, b))

    # Pass 2: remove small repeated islands.
    sorted_result = sorted(result)
    runs: list[tuple[int, int]] = []
    run_start = sorted_result[0]
    for i in range(1, len(sorted_result)):
        if sorted_result[i] != sorted_result[i - 1] + 1:
            runs.append((run_start, sorted_result[i - 1]))
            run_start = sorted_result[i]
    runs.append((run_start, sorted_result[-1]))

    for start, end in runs:
        if end - start + 1 <= min_repeated_island:
            result.difference_update(range(start, end + 1))

    return result


def _build_segment(lookup: list[dict[str, Any]], start: int, end: int, is_repeated: bool) -> dict[str, Any]:
    texts = [lookup[i]["text"] for i in range(start, end + 1) if lookup[i].get("text")]
    first = lookup[start] if start < len(lookup) else {}
    last = lookup[end] if end < len(lookup) else {}

    seg: dict[str, Any] = {
        "type": "repeated" if is_repeated else "unique",
        "start_index": start,
        "end_index": end,
        "entry_count": end - start + 1,
        "texts": texts,
        "start_ms": first.get("start_ms", 0) or 0,
        "end_ms": last.get("end_ms", 0) or 0,
    }
    if first.get("start_formatted"):
        seg["start_formatted"] = first["start_formatted"]
    if last.get("end_formatted"):
        seg["end_formatted"] = last["end_formatted"]
    return seg


def run_extract_unique(config: dict[str, Any]) -> None:
    lookup = build_entry_lookup(config["source"])
    total = len(lookup)

    with open(config["result"]) as f:
        result_json = json.load(f)

    raw_repeated = collect_repeated_indices(result_json)
    repeated = _consolidate_repeated(
        raw_repeated,
        max_unique_gap=config.get("max_unique_gap", 3),
        min_repeated_island=config.get("min_repeated_island", 2),
    )
    print(
        f"{len(raw_repeated)} raw / {len(repeated)} consolidated / {total} total entries",
        file=sys.stderr,
    )

    segments = []
    if total > 0:
        seg_start = 0
        seg_repeated = 0 in repeated

        for i in range(1, total):
            is_rep = i in repeated
            if is_rep != seg_repeated:
                segments.append(_build_segment(lookup, seg_start, i - 1, seg_repeated))
                seg_start = i
                seg_repeated = is_rep
        segments.append(_build_segment(lookup, seg_start, total - 1, seg_repeated))

    print(json.dumps(segments, indent=2, ensure_ascii=False))


def run_enrich(config: dict[str, Any]) -> None:
    lookup = build_entry_lookup(config["source"])
    print(f"Loaded {len(lookup)} entries from source for enrichment lookup", file=sys.stderr)

    with open(config["result"]) as f:
        result_json = json.load(f)

    # Inject total_duration_secs at top level
    if isinstance(result_json, dict) and lookup:
        first = lookup[0]
        last = lookup[-1]
        if first.get("start_ms") is not None and last.get("end_ms") is not None:
            result_json["total_duration_secs"] = (last["end_ms"] - first["start_ms"]) / 1000.0

    enrich_value(result_json, lookup)
    print(json.dumps(result_json, indent=2, ensure_ascii=False))
