#!/usr/bin/env python3
"""Extract segments from ExtractUnique JSON output.

Supports station-specific classification of repeated segments into categories
(ads, weather, station bumpers, etc.) via pluggable classifier modules in
the stations/ directory. Unique segments pass through as-is.

Short segments -> consolidated markdown files per category.
Long segments  -> individual text files in category subfolders.
"""

import importlib
import json
import os
import sys
import argparse


# ---------------------------------------------------------------------------
# Station classifier loading
# ---------------------------------------------------------------------------

def _discover_stations() -> list[str]:
    """Find available station modules in stations/."""
    stations_dir = os.path.join(os.path.dirname(__file__), "stations")
    if not os.path.isdir(stations_dir):
        return []
    return [
        f[:-3] for f in os.listdir(stations_dir)
        if f.endswith(".py") and f != "__init__.py"
    ]


def _load_classifier(station: str):
    """Load the classify() function from stations/<station>.py."""
    mod = importlib.import_module(f"stations.{station}")
    return mod.classify


# ---------------------------------------------------------------------------
# Output helpers
# ---------------------------------------------------------------------------

def sanitize_ts(ts: str) -> str:
    return ts.replace(":", "_").split(".")[0]


def format_ts(ts: str) -> str:
    return ts.split(".")[0]


def seg_filename(seg: dict) -> str:
    start_ts = sanitize_ts(seg.get("start_formatted", f"idx{seg['start_index']}"))
    end_ts = sanitize_ts(seg.get("end_formatted", f"idx{seg['end_index']}"))
    tag = seg["type"][0]
    return f"{start_ts}--{end_ts}_{tag}_{seg['entry_count']}entries.txt"


def seg_header(seg: dict) -> str:
    start = format_ts(seg.get("start_formatted", "?"))
    end = format_ts(seg.get("end_formatted", "?"))
    return f"{start} - {end} ({seg['entry_count']} entries)"


def write_consolidated_md(filepath: str, segments: list, title: str):
    with open(filepath, "w") as f:
        f.write(f"# {title}\n\n")
        for seg in segments:
            f.write(f"## {seg_header(seg)}\n\n")
            for text in seg["texts"]:
                f.write(text + "\n\n")
            f.write("---\n\n")


def write_individual_files(folder: str, segments: list):
    os.makedirs(folder, exist_ok=True)
    for seg in segments:
        filepath = os.path.join(folder, seg_filename(seg))
        with open(filepath, "w") as f:
            for text in seg["texts"]:
                f.write(text + "\n")


def output_category(outdir: str, category: str, segments: list, long_threshold: int):
    """Write one category: short -> md, long -> folder."""
    short = [s for s in segments if s["entry_count"] < long_threshold]
    long = [s for s in segments if s["entry_count"] >= long_threshold]

    if short:
        md_path = os.path.join(outdir, f"{category}_short.md")
        write_consolidated_md(md_path, short, f"{category} (short)")
        print(f"  {len(short):>4} short -> {md_path}", file=sys.stderr)

    if long:
        folder = os.path.join(outdir, f"{category}_long")
        write_individual_files(folder, long)
        print(f"  {len(long):>4} long  -> {folder}/", file=sys.stderr)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    available = _discover_stations()

    parser = argparse.ArgumentParser(description="Extract segments to files")
    parser.add_argument("segments_json", help="Path to segments JSON from extract-unique")
    parser.add_argument(
        "--station",
        choices=available or None,
        default=None,
        help=f"Station-specific classifier (available: {', '.join(available) or 'none'})",
    )
    parser.add_argument(
        "--min-entries",
        type=int,
        default=3,
        help="Minimum entry count to include (default: 3)",
    )
    parser.add_argument(
        "--long-threshold",
        type=int,
        default=10,
        help="Segments with >= this many entries go to individual files (default: 10)",
    )
    parser.add_argument(
        "--outdir",
        default=None,
        help="Output directory (default: <segments_json_stem>_out/)",
    )
    args = parser.parse_args()

    with open(args.segments_json) as f:
        segments = json.load(f)

    segments = [s for s in segments if s["entry_count"] >= args.min_entries]

    if not segments:
        print("No segments match the criteria.", file=sys.stderr)
        sys.exit(0)

    outdir = args.outdir or os.path.splitext(args.segments_json)[0] + "_out"
    os.makedirs(outdir, exist_ok=True)

    unique_segs = [s for s in segments if s["type"] == "unique"]
    repeated_segs = [s for s in segments if s["type"] == "repeated"]

    # Unique segments: output as-is
    if unique_segs:
        print(f"Unique: {len(unique_segs)} segments", file=sys.stderr)
        output_category(outdir, "unique", unique_segs, args.long_threshold)

    # Repeated segments: classify if station is specified
    if repeated_segs:
        if args.station:
            classify = _load_classifier(args.station)
            categorized: dict[str, list] = {}
            for seg in repeated_segs:
                cat = classify(seg)
                categorized.setdefault(cat, []).append(seg)

            for cat in sorted(categorized):
                cat_segs = categorized[cat]
                print(f"Repeated/{cat}: {len(cat_segs)} segments", file=sys.stderr)
                output_category(outdir, f"repeated_{cat}", cat_segs, args.long_threshold)
        else:
            print(f"Repeated: {len(repeated_segs)} segments", file=sys.stderr)
            output_category(outdir, "repeated", repeated_segs, args.long_threshold)


if __name__ == "__main__":
    main()
