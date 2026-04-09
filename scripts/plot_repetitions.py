#!/usr/bin/env python3
"""Plot repetition counts from enriched JSON as a horizontal bar chart."""

import json
import sys

import matplotlib
import matplotlib.pyplot as plt
from matplotlib.patches import Patch

matplotlib.rcParams["font.family"] = ["Arial Unicode MS", "sans-serif"]

TRUNCATE_LEN = 30


def truncate(text, max_len=TRUNCATE_LEN):
    return text[:max_len] + "…" if len(text) > max_len else text


def main():
    path = sys.argv[1] if len(sys.argv) > 1 else "tmp/kmrb_monday_enriched.json"
    with open(path) as f:
        data = json.load(f)

    items = []  # (label, count, category)

    for dup in data.get("exact_duplicates", []):
        items.append((truncate(dup["canonical_text"]), dup["count"], "Exact duplicate"))

    for cluster in data.get("near_duplicates", []):
        text = cluster["representative_text"]
        items.append((truncate(text), len(cluster["members"]), "Near duplicate"))

    for ng in data.get("ngrams", []):
        items.append((truncate(ng["ngram"]), ng["count"], "Repeated n-gram"))

    for seq in data.get("repeated_sequences", []):
        text = " / ".join(seq.get("entry_texts", [f"Sequence (len {seq.get('length', '?')})"]))
        count = len(seq.get("occurrences", []))
        items.append((truncate(text), count, "Repeated sequence"))

    if not items:
        print("No repetitions found.")
        return

    items.sort(key=lambda x: x[1], reverse=True)

    labels = [it[0] for it in items]
    counts = [it[1] for it in items]
    categories = [it[2] for it in items]

    colors = {
        "Exact duplicate": "#e74c3c",
        "Near duplicate": "#f39c12",
        "Repeated n-gram": "#3498db",
        "Repeated sequence": "#2ecc71",
    }
    bar_colors = [colors[c] for c in categories]

    fig, ax = plt.subplots(figsize=(14, max(6, len(items) * 0.8)))
    y_pos = range(len(items))
    ax.barh(y_pos, counts, color=bar_colors)
    ax.set_yticks(y_pos)
    ax.set_yticklabels(labels)
    ax.invert_yaxis()
    ax.set_xlabel("Repetition count")
    ax.set_title("Repetitions")
    ax.xaxis.set_major_locator(plt.MaxNLocator(integer=True))

    legend_items = [Patch(facecolor=colors[c], label=c) for c in colors if c in set(categories)]
    ax.legend(handles=legend_items, loc="lower right")

    plt.tight_layout()
    out = path.replace("_enriched.json", "_repetitions.svg")
    plt.savefig(out)
    print(f"Saved to {out}")


if __name__ == "__main__":
    main()
