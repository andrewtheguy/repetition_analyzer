"""Output formatting for human-readable and JSON reports."""

import json


def truncate(s: str, max_len: int) -> str:
    if len(s) <= max_len:
        return s
    return s[:max_len] + "..."


def print_report(data: dict, top_n: int) -> None:
    entries = data["entries"]
    duplicates = data["duplicates"]
    near_dupes = data["near_dupes"]
    ngrams = data["ngrams"]
    sequences = data["sequences"]
    near_seqs = data["near_seqs"]

    print()
    print("=" * 70)
    print("  REPETITION ANALYSIS REPORT")
    print(f"  File: {data['file_path']}")
    print(f"  Entries: {len(entries)}")
    print("=" * 70)

    # Section 1: Exact Duplicates
    print()
    print(f"--- 1. EXACT DUPLICATES ({len(duplicates)} texts appear 2+ times) ---")
    print()

    for i, group in enumerate(duplicates[:top_n]):
        print(f"  {i+1:>3}. {group['count']:>3}x | \"{truncate(group['canonical_text'], 120)}\"")
        first_id = group["indices"][0][1]
        last_id = group["indices"][-1][1]
        print(f"        First: id={first_id} | Last: id={last_id}")
        print()

    # Section 2: Near-Duplicates
    print(f"--- 2. NEAR-DUPLICATE CLUSTERS ({len(near_dupes)} clusters) ---")
    print()

    for i, cluster in enumerate(near_dupes[:top_n]):
        print(f"  {i+1:>3}. Cluster ({cluster['total_count']} variants): \"{truncate(cluster['representative_text'], 100)}\"")
        for idx, member in enumerate(cluster["members"][:5]):
            print(f"        [{idx+1}] \"{truncate(member[2], 100)}\"")
        if len(cluster["members"]) > 5:
            print(f"        ... and {len(cluster['members']) - 5} more variants")
        print()

    # Section 3: N-grams
    print("--- 3. MOST REPEATED PHRASES ---")
    print()

    for i, ng in enumerate(ngrams[:top_n]):
        print(f"  {i+1:>3}. {ng['count']:>4}x ({ng['n']}-gram, {len(ng['entry_indices'])} entries) | \"{ng['ngram']}\"")
    print()

    # Section 4: Repeated Sequences
    print(f"--- 4. REPEATED SEGMENT BLOCKS ({len(sequences)} unique blocks) ---")
    print()

    for i, seq in enumerate(sequences[:top_n]):
        print(f"  {i+1:>3}. {len(seq['occurrences']):>3}x | {seq['length']}-entry block")
        for j, text in enumerate(seq["entry_texts"]):
            print(f"        [{j+1}] \"{truncate(text, 100)}\"")
        indices = [str(o["start_index"]) for o in seq["occurrences"][:10]]
        line = f"        At index: {', '.join(indices)}"
        if len(seq["occurrences"]) > 10:
            line += f" ... +{len(seq['occurrences']) - 10} more"
        print(line)
        print()

    # Section 5: Near-Duplicate Sequences
    print(f"--- 5. NEAR-DUPLICATE SEGMENT BLOCKS ({len(near_seqs)} unique patterns) ---")
    print()

    for i, seq in enumerate(near_seqs[:top_n]):
        print(f"  {i+1:>3}. {len(seq['occurrences']):>3}x | {seq['length']}-entry block | avg similarity: {seq['avg_similarity']*100:.1f}%")
        for occ_idx, occ in enumerate(seq["occurrences"][:3]):
            print(f"        Occurrence {occ_idx+1} (index {occ['start_index']}):")
            for j, text in enumerate(occ["entry_texts"]):
                print(f"          [{j+1}] \"{truncate(text, 90)}\"")
        if len(seq["occurrences"]) > 3:
            print(f"        ... +{len(seq['occurrences']) - 3} more occurrences")
        print()

    # Section 6: Summary
    print("--- 6. SUMMARY ---")
    print()
    print(f"  Exact duplicate groups:       {len(duplicates)}")
    print(f"  Near-duplicate clusters:      {len(near_dupes)}")
    print(f"  Near-duplicate seq. patterns: {len(near_seqs)}")

    if ngrams:
        top_ng = ngrams[0]
        print(f"  Most repeated phrase:         \"{truncate(top_ng['ngram'], 60)}\" ({top_ng['count']}x)")

    if sequences:
        top_seq = sequences[0]
        print(f"  Most repeated block:          {top_seq['length']}-entry block ({len(top_seq['occurrences'])}x)")

    print()
    print("=" * 70)


def print_json_report(data: dict) -> None:
    report = {
        "file_path": data["file_path"],
        "total_entries": len(data["entries"]),
        "exact_duplicates": data["duplicates"],
        "near_duplicates": data["near_dupes"],
        "ngrams": data["ngrams"],
        "repeated_sequences": data["sequences"],
        "near_duplicate_sequences": data["near_seqs"],
    }
    print(json.dumps(report, indent=2, ensure_ascii=False))
