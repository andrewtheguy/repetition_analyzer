"""Segment extraction: write segments to markdown/text files with optional station classification."""

import json
from pathlib import Path

from .stations import classify


def _sanitize_ts(ts: str) -> str:
    return ts.split(".")[0]


def _filename(seg: dict) -> str:
    start = _sanitize_ts(seg.get("start_formatted", "?")).replace(":", "_")
    end = _sanitize_ts(seg.get("end_formatted", "?")).replace(":", "_")
    tag = seg["type"][0]
    return f"{start}--{end}_{tag}_{seg['entry_count']}entries.txt"


def _header(seg: dict) -> str:
    start = _sanitize_ts(seg.get("start_formatted", "?"))
    end = _sanitize_ts(seg.get("end_formatted", "?"))
    return f"{start} - {end} ({seg['entry_count']} entries)"


def _duration_secs(seg: dict) -> float:
    return (seg.get("end_ms", 0) - seg.get("start_ms", 0)) / 1000.0


def _text_blob(seg: dict) -> str:
    return " ".join(seg.get("texts", [])).lower()


def _write_consolidated_md(path: Path, segments: list[dict], title: str) -> None:
    with open(path, "w") as f:
        f.write(f"# {title}\n\n")
        for seg in segments:
            f.write(f"## {_header(seg)}\n\n")
            for text in seg.get("texts", []):
                f.write(f"{text}\n\n")
            f.write("---\n\n")


def _write_individual_files(folder: Path, segments: list[dict]) -> None:
    folder.mkdir(parents=True, exist_ok=True)
    for seg in segments:
        with open(folder / _filename(seg), "w") as f:
            for text in seg.get("texts", []):
                f.write(f"{text}\n")


def _output_category(outdir: Path, category: str, segments: list[dict], long_threshold: int) -> None:
    import sys
    short = [s for s in segments if s["entry_count"] < long_threshold]
    long = [s for s in segments if s["entry_count"] >= long_threshold]

    if short:
        md_path = outdir / f"{category}_short.md"
        _write_consolidated_md(md_path, short, f"{category} (short)")
        print(f"  {len(short):>4} short -> {md_path}", file=sys.stderr)

    if long:
        folder = outdir / f"{category}_long"
        _write_individual_files(folder, long)
        print(f"  {len(long):>4} long  -> {folder}/", file=sys.stderr)


def run_extract_segments(config: dict) -> None:
    import sys

    with open(config["segments"]) as f:
        all_segments = json.load(f)

    segments = [s for s in all_segments if s.get("entry_count", 0) >= config.get("min_entries", 3)]

    if not segments:
        print("No segments match the criteria.", file=sys.stderr)
        return

    outdir = Path(config["outdir"])
    outdir.mkdir(parents=True, exist_ok=True)

    unique = [s for s in segments if s.get("type") == "unique"]
    repeated = [s for s in segments if s.get("type") == "repeated"]

    if unique:
        print(f"Unique: {len(unique)} segments", file=sys.stderr)
        _output_category(outdir, "unique", unique, config.get("long_threshold", 10))

    if repeated:
        station = config.get("station")
        if station:
            from collections import defaultdict
            categorized: dict[str, list[dict]] = defaultdict(list)
            for seg in repeated:
                cat = classify(station, seg, _duration_secs, _text_blob)
                categorized[cat].append(seg)

            for cat in sorted(categorized):
                cat_segs = categorized[cat]
                print(f"Repeated/{cat}: {len(cat_segs)} segments", file=sys.stderr)
                _output_category(outdir, f"repeated_{cat}", cat_segs, config.get("long_threshold", 10))
        else:
            print(f"Repeated: {len(repeated)} segments", file=sys.stderr)
            _output_category(outdir, "repeated", repeated, config.get("long_threshold", 10))
