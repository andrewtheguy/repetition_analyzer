"""Segment extraction: write segments to markdown/text files."""

import json
import sys
from pathlib import Path
from typing import Any


def _sanitize_ts(ts: str) -> str:
    return ts.split(".")[0]


def _filename(seg: dict[str, Any]) -> str:
    start = _sanitize_ts(seg.get("start_formatted", "?")).replace(":", "_")
    end = _sanitize_ts(seg.get("end_formatted", "?")).replace(":", "_")
    tag = seg["type"][0]
    return f"{start}--{end}_{tag}_{seg['entry_count']}entries.txt"


def _header(seg: dict[str, Any]) -> str:
    start = _sanitize_ts(seg.get("start_formatted", "?"))
    end = _sanitize_ts(seg.get("end_formatted", "?"))
    return f"{start} - {end} ({seg['entry_count']} entries)"


def _write_consolidated_md(path: Path, segments: list[dict[str, Any]], title: str) -> None:
    with open(path, "w") as f:
        f.write(f"# {title}\n\n")
        for seg in segments:
            f.write(f"## {_header(seg)}\n\n")
            for text in seg.get("texts", []):
                f.write(f"{text}\n\n")
            f.write("---\n\n")


def _write_individual_files(folder: Path, segments: list[dict[str, Any]]) -> None:
    folder.mkdir(parents=True, exist_ok=True)
    for seg in segments:
        with open(folder / _filename(seg), "w") as f:
            for text in seg.get("texts", []):
                f.write(f"{text}\n")


def _output_category(outdir: Path, category: str, segments: list[dict[str, Any]], long_threshold: int) -> None:
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


def run_extract_segments(config: dict[str, Any]) -> None:
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
        print(f"Repeated: {len(repeated)} segments", file=sys.stderr)
        _output_category(outdir, "repeated", repeated, config.get("long_threshold", 10))
