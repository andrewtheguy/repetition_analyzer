"""Detect rebroadcast blocks: long contiguous segments that replay earlier content."""

import csv
import json
import sys
import time
from collections import defaultdict
from difflib import SequenceMatcher
from typing import Any


def _normalize(text: str) -> str:
    """Normalize text: lowercase, keep alphanumerics and apostrophes, collapse spaces."""
    lower = text.lower()
    cleaned = "".join(c if c.isalnum() or c == "'" else " " for c in lower)
    return " ".join(cleaned.split())


def _similarity(a: str, b: str) -> float:
    """Compute similarity ratio between two strings."""
    if not a and not b:
        return 1.0
    return SequenceMatcher(None, a, b).ratio()


def detect_rebroadcasts(
    texts: list[str],
    min_block_length: int = 10,
    similarity_threshold: float = 0.7,
    min_gap: int = 50,
    merge_gap: int = 5,
) -> list[dict[str, Any]]:
    """Detect rebroadcast blocks in a list of text entries.

    A rebroadcast is a contiguous block of entries where each entry closely
    matches an entry from an earlier contiguous block, preserving order.

    Args:
        texts: List of text strings (one per entry).
        min_block_length: Minimum matched entries to count as a rebroadcast.
        similarity_threshold: Minimum similarity (0-1) for a match.
        min_gap: Minimum index distance between original and rebroadcast.
        merge_gap: Merge blocks whose rebroadcast ranges are within this many entries.
    """
    n = len(texts)
    normed = [_normalize(t) for t in texts]

    # Build prefix index for candidate lookup.
    # Use first 15 chars of normalized text as bucket key.
    prefix_len = 15
    prefix_index: dict[str, list[int]] = defaultdict(list)
    for i, norm in enumerate(normed):
        key = norm[:prefix_len]
        if len(key) >= 5:
            prefix_index[key].append(i)

    # Find seed matches: for each later entry, find a matching earlier entry.
    seeds: dict[int, tuple[int, float]] = {}

    for indices in prefix_index.values():
        if len(indices) < 2:
            continue
        for a_pos in range(len(indices)):
            j = indices[a_pos]
            for b_pos in range(a_pos + 1, len(indices)):
                i = indices[b_pos]
                if i - j < min_gap:
                    continue
                len_i = len(normed[i])
                len_j = len(normed[j])
                if len_i < 10 or len_j < 10:
                    continue
                if min(len_i, len_j) < 0.5 * max(len_i, len_j):
                    continue
                sim = _similarity(normed[i], normed[j])
                if sim >= similarity_threshold:
                    if i not in seeds or sim > seeds[i][1]:
                        seeds[i] = (j, sim)

    # Extend seed matches into contiguous blocks.
    blocks: list[dict[str, Any]] = []
    visited: set[int] = set()

    for i_start in sorted(seeds.keys()):
        if i_start in visited:
            continue
        j_start, seed_sim = seeds[i_start]

        # Collect matched pairs by extending forward from the seed.
        matches: list[tuple[int, int, float]] = [(i_start, j_start, seed_sim)]
        j_expected = j_start + 1
        consecutive_misses = 0
        max_misses = 3

        i_curr = i_start + 1
        while i_curr < n and consecutive_misses < max_misses:
            best_sim = 0.0
            best_j = -1
            for delta in range(-2, 4):
                test_j = j_expected + delta
                if test_j < 0 or test_j >= n or test_j >= i_curr:
                    continue
                len_i = len(normed[i_curr])
                len_j = len(normed[test_j])
                if len_i < 10 or len_j < 10:
                    continue
                if min(len_i, len_j) < 0.5 * max(len_i, len_j):
                    continue
                sim = _similarity(normed[i_curr], normed[test_j])
                if sim > best_sim:
                    best_sim = sim
                    best_j = test_j

            if best_sim >= similarity_threshold:
                matches.append((i_curr, best_j, best_sim))
                j_expected = best_j + 1
                consecutive_misses = 0
            else:
                consecutive_misses += 1
                j_expected += 1

            i_curr += 1

        if len(matches) >= min_block_length:
            avg_sim = sum(s for _, _, s in matches) / len(matches)
            blocks.append({
                "rebroadcast_start_index": matches[0][0],
                "rebroadcast_end_index": matches[-1][0],
                "original_start_index": matches[0][1],
                "original_end_index": matches[-1][1],
                "num_matched_entries": len(matches),
                "avg_similarity": round(avg_sim, 4),
            })
            for i, _, _ in matches:
                visited.add(i)

    # Merge blocks whose rebroadcast ranges are close together.
    if merge_gap > 0 and len(blocks) > 1:
        merged: list[dict[str, Any]] = [blocks[0]]
        for blk in blocks[1:]:
            prev = merged[-1]
            gap = blk["rebroadcast_start_index"] - prev["rebroadcast_end_index"]
            if gap <= merge_gap:
                total_matched = prev["num_matched_entries"] + blk["num_matched_entries"]
                prev_weight = prev["num_matched_entries"]
                blk_weight = blk["num_matched_entries"]
                prev["rebroadcast_end_index"] = blk["rebroadcast_end_index"]
                prev["original_end_index"] = max(prev["original_end_index"], blk["original_end_index"])
                prev["original_start_index"] = min(prev["original_start_index"], blk["original_start_index"])
                prev["avg_similarity"] = round(
                    (prev["avg_similarity"] * prev_weight + blk["avg_similarity"] * blk_weight) / total_matched,
                    4,
                )
                prev["num_matched_entries"] = total_matched
            else:
                merged.append(blk)
        blocks = merged

    return blocks


def run_detect_rebroadcast(config: dict[str, Any]) -> None:
    """CLI entry point for rebroadcast detection."""
    start = time.time()
    path = config["file"]

    print(f"Loading {path}...", file=sys.stderr)

    rows: list[dict[str, str]] = []
    with open(path, newline="") as f:
        reader = csv.DictReader(f)
        for row in reader:
            rows.append(row)

    texts = [row["text"] for row in rows]
    print(f"Loaded {len(texts)} entries ({time.time() - start:.2f}s)", file=sys.stderr)

    t = time.time()
    blocks = detect_rebroadcasts(
        texts,
        min_block_length=config.get("min_block_length", 10),
        similarity_threshold=config.get("similarity_threshold", 0.7),
        min_gap=config.get("min_gap", 50),
        merge_gap=config.get("merge_gap", 5),
    )
    print(f"Found {len(blocks)} rebroadcast block(s) ({time.time() - t:.2f}s)", file=sys.stderr)

    # Enrich with timestamps.
    for block in blocks:
        rb_start = block["rebroadcast_start_index"]
        rb_end = block["rebroadcast_end_index"]
        orig_start = block["original_start_index"]
        orig_end = block["original_end_index"]

        block["rebroadcast_start_time"] = rows[rb_start]["start_formatted"]
        block["rebroadcast_end_time"] = rows[rb_end]["end_formatted"]
        block["original_start_time"] = rows[orig_start]["start_formatted"]
        block["original_end_time"] = rows[orig_end]["end_formatted"]

        rb_duration_ms = int(rows[rb_end]["end_ms"]) - int(rows[rb_start]["start_ms"])
        block["rebroadcast_duration_secs"] = round(rb_duration_ms / 1000, 1)

    total_rb_entries = sum(
        b["rebroadcast_end_index"] - b["rebroadcast_start_index"] + 1
        for b in blocks
    )
    total_rb_duration = sum(b["rebroadcast_duration_secs"] for b in blocks)

    result = {
        "file_path": path,
        "total_entries": len(texts),
        "total_rebroadcast_entries": total_rb_entries,
        "total_rebroadcast_duration_secs": round(total_rb_duration, 1),
        "rebroadcast_blocks": blocks,
    }

    print(json.dumps(result, indent=2, ensure_ascii=False))

    elapsed = time.time() - start
    print(f"Done in {elapsed:.2f}s", file=sys.stderr)
