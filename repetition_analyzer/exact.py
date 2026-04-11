"""Exact duplicate detection (Python implementation using native_helper.normalize)."""

from collections import defaultdict

from native_helper import normalize

from .parse import Entry


def find_exact_duplicates(entries: list[Entry]) -> list[dict]:
    """Group entries by normalized text, returning groups with 2+ members."""
    groups: dict[str, list[int]] = defaultdict(list)

    for entry in entries:
        norm = normalize(entry.text)
        groups[norm].append(entry.index)

    result = []
    for indices in groups.values():
        if len(indices) < 2:
            continue
        canonical_text = entries[indices[0]].text
        result.append({
            "canonical_text": canonical_text,
            "count": len(indices),
            "indices": [(i, entries[i].id) for i in indices],
        })

    result.sort(key=lambda g: g["count"], reverse=True)
    return result
