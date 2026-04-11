"""CSV parsing for the canonical format: id,text,start_ms,end_ms,start_formatted,end_formatted."""

import csv
from dataclasses import dataclass


@dataclass
class Entry:
    index: int
    id: str
    text: str


def parse_csv(path: str) -> list[Entry]:
    """Parse a preprocessed CSV file into a list of Entry objects."""
    entries: list[Entry] = []
    seen_ids: set[str] = set()

    with open(path, newline="") as f:
        reader = csv.DictReader(f)
        for line_num, row in enumerate(reader):
            entry_id = row["id"]
            text = row["text"]

            if entry_id in seen_ids:
                raise ValueError(f"line {line_num + 1}: duplicate id '{entry_id}'")
            seen_ids.add(entry_id)

            entries.append(Entry(index=line_num, id=entry_id, text=text))

    return entries


def entries_to_tuples(entries: list[Entry]) -> list[tuple[int, str, str]]:
    """Convert entries to the tuple format expected by native_helper."""
    return [(e.index, e.id, e.text) for e in entries]
