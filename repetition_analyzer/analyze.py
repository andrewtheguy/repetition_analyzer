"""Orchestrate all analysis steps: parse CSV, run algorithms, produce report."""

import json
import sys
import time

import native_helper

from .exact import find_exact_duplicates
from .parse import entries_to_tuples, parse_csv
from .report import print_json_report, print_report


def run_analyze(config: dict) -> None:
    start = time.time()

    # Parse
    t = time.time()
    print(f"Parsing {config['file']}...", file=sys.stderr)
    entries = parse_csv(config["file"])
    print(f"Loaded {len(entries)} entries ({time.time() - t:.2f}s)", file=sys.stderr)

    tuples = entries_to_tuples(entries)

    # Exact duplicates (Python)
    t = time.time()
    duplicates = find_exact_duplicates(entries)
    print(f"Found {len(duplicates)} duplicate groups ({time.time() - t:.2f}s)", file=sys.stderr)

    # Near-duplicates (Rust)
    t = time.time()
    near_dupes = native_helper.find_near_duplicates(tuples, config.get("similarity_threshold", 0.85))
    print(f"Found {len(near_dupes)} near-duplicate clusters ({time.time() - t:.2f}s)", file=sys.stderr)

    # N-grams (Rust)
    t = time.time()
    ngrams = native_helper.extract_ngrams(
        tuples,
        config.get("min_ngram", 3),
        config.get("max_ngram", 8),
        config.get("min_count", 3),
    )
    print(f"Found {len(ngrams)} significant n-grams ({time.time() - t:.2f}s)", file=sys.stderr)

    # Repeated sequences (Rust)
    t = time.time()
    sequences = native_helper.find_repeated_sequences(
        tuples,
        config.get("min_seq_len", 2),
        config.get("max_seq_len", 8),
        config.get("min_seq_occurrences", 2),
    )
    print(f"Found {len(sequences)} repeated sequence patterns ({time.time() - t:.2f}s)", file=sys.stderr)

    # Near-duplicate sequences (Rust)
    t = time.time()
    near_seqs = native_helper.find_near_duplicate_sequences(
        tuples,
        config.get("min_seq_len", 2),
        config.get("max_seq_len", 8),
        config.get("seq_similarity_threshold", 0.80),
        config.get("min_seq_occurrences", 2),
        json.dumps(sequences),
    )
    print(f"Found {len(near_seqs)} near-duplicate sequence patterns ({time.time() - t:.2f}s)", file=sys.stderr)

    elapsed = time.time() - start
    print(f"Analysis complete in {elapsed:.2f}s", file=sys.stderr)

    data = {
        "file_path": config["file"],
        "entries": [{"index": e.index, "id": e.id, "text": e.text} for e in entries],
        "duplicates": duplicates,
        "near_dupes": near_dupes,
        "ngrams": ngrams,
        "sequences": sequences,
        "near_seqs": near_seqs,
    }

    fmt = config.get("format", "human")
    if fmt == "json":
        print_json_report(data)
    else:
        print_report(data, config.get("top_n", 20))
