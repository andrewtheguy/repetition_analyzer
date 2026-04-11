"""Segment extraction: write segments to markdown/text and HTML files."""

import html
import json
import sys
from pathlib import Path
from typing import Any

TRUNCATE_LEN = 60


def _offset_ts(ts: str, offset_ms: int) -> str:
    """Add offset to a HH:MM:SS.mmm timestamp and strip millis."""
    if not ts or ts == "?":
        return ts
    from .preprocess import formatted_to_ms, ms_to_formatted
    ms = formatted_to_ms(ts)
    if ms is None:
        return ts.split(".")[0]
    return ms_to_formatted(ms + offset_ms).split(".")[0]


def _filename(seg: dict[str, Any], offset_ms: int = 0) -> str:
    start = _offset_ts(seg.get("start_formatted", "?"), offset_ms).replace(":", "_")
    end = _offset_ts(seg.get("end_formatted", "?"), offset_ms).replace(":", "_")
    tag = seg["type"][0]
    rep = f"_x{seg['rep_count']}" if seg.get("rep_count") else ""
    return f"{start}--{end}_{tag}_{seg['entry_count']}entries{rep}.txt"


def _header(seg: dict[str, Any], offset_ms: int = 0) -> str:
    start = _offset_ts(seg.get("start_formatted", "?"), offset_ms)
    end = _offset_ts(seg.get("end_formatted", "?"), offset_ms)
    rep = f", {seg['rep_count']}x" if seg.get("rep_count") else ""
    return f"{start} - {end} ({seg['entry_count']} entries{rep})"


def _write_consolidated_md(path: Path, segments: list[dict[str, Any]], title: str, offset_ms: int = 0) -> None:
    with open(path, "w") as f:
        f.write(f"# {title}\n\n")
        for seg in segments:
            f.write(f"## {_header(seg, offset_ms)}\n\n")
            for text in seg.get("texts", []):
                f.write(f"{text}\n\n")
            f.write("---\n\n")


def _write_individual_files(folder: Path, segments: list[dict[str, Any]], offset_ms: int = 0) -> None:
    folder.mkdir(parents=True, exist_ok=True)
    for seg in segments:
        with open(folder / _filename(seg, offset_ms), "w") as f:
            for text in seg.get("texts", []):
                f.write(f"{text}\n")


def _output_category(outdir: Path, category: str, segments: list[dict[str, Any]], long_threshold: int, offset_ms: int = 0) -> None:
    short = [s for s in segments if s["entry_count"] < long_threshold]
    long = [s for s in segments if s["entry_count"] >= long_threshold]

    if short:
        md_path = outdir / f"{category}_short.md"
        _write_consolidated_md(md_path, short, f"{category} (short)", offset_ms)
        print(f"  {len(short):>4} short -> {md_path}", file=sys.stderr)

    if long:
        folder = outdir / f"{category}_long"
        _write_individual_files(folder, long, offset_ms)
        print(f"  {len(long):>4} long  -> {folder}/", file=sys.stderr)


def _render_segments_html(segments: list[dict[str, Any]], title: str, offset_ms: int = 0) -> str:
    rows = ""
    for i, seg in enumerate(segments):
        start = _offset_ts(seg.get("start_formatted", "?"), offset_ms)
        end = _offset_ts(seg.get("end_formatted", "?"), offset_ms)
        count = seg["entry_count"]
        rep_count = seg.get("rep_count", 0)
        duration_ms = seg.get("end_ms", 0) - seg.get("start_ms", 0)
        duration_min = duration_ms / 60000

        rep_badge = f'<span class="seg-rep">{rep_count}x</span>' if rep_count else ""

        preview_lines = ""
        texts = seg.get("texts", [])
        for t in texts[:3]:
            escaped = html.escape(t[:TRUNCATE_LEN])
            if len(t) > TRUNCATE_LEN:
                escaped += "\u2026"
            preview_lines += f'<div class="preview-line">{escaped}</div>\n'
        if len(texts) > 3:
            preview_lines += f'<div class="preview-more">... +{len(texts) - 3} more</div>\n'

        full_lines = ""
        for t in texts:
            full_lines += f"<p>{html.escape(t)}</p>\n"

        rows += f"""<div class="segment" onclick="toggle({i})">
  <div class="seg-header">
    <span class="seg-time">{start} \u2013 {end}</span>
    <span class="seg-count">{count} entries</span>
    <span class="seg-duration">{duration_min:.1f} min</span>
    {rep_badge}
  </div>
  <div class="seg-preview">{preview_lines}</div>
  <div class="seg-full" id="full-{i}">{full_lines}</div>
</div>
"""

    return f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>{html.escape(title)}</title>
<style>
  body {{ font-family: system-ui, -apple-system, sans-serif; margin: 2em; background: #fafafa; max-width: 900px; }}
  h1 {{ font-size: 1.4em; }}
  .summary {{ color: #666; margin-bottom: 1.5em; font-size: 0.9em; }}
  .segment {{ border: 1px solid #ddd; border-radius: 6px; margin-bottom: 8px; padding: 10px 14px; background: #fff; cursor: pointer; }}
  .segment:hover {{ border-color: #aaa; }}
  .seg-header {{ display: flex; gap: 16px; align-items: baseline; margin-bottom: 4px; }}
  .seg-time {{ font-weight: 600; font-size: 0.95em; }}
  .seg-count {{ color: #666; font-size: 0.85em; }}
  .seg-duration {{ color: #999; font-size: 0.85em; }}
  .seg-rep {{ color: #e74c3c; font-size: 0.85em; font-weight: 600; }}
  .seg-preview {{ font-size: 0.85em; color: #444; }}
  .preview-line {{ white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }}
  .preview-more {{ color: #999; font-style: italic; }}
  .seg-full {{ display: none; font-size: 0.85em; line-height: 1.6; margin-top: 8px; border-top: 1px solid #eee; padding-top: 8px; }}
  .seg-full.open {{ display: block; }}
  .seg-full p {{ margin: 0.3em 0; }}
</style>
</head>
<body>
<h1>{html.escape(title)}</h1>
<div class="summary">{len(segments)} segments</div>
{rows}
<script>
function toggle(i) {{
  var el = document.getElementById('full-' + i);
  el.classList.toggle('open');
}}
</script>
</body>
</html>"""


def run_extract_segments(config: dict[str, Any]) -> None:
    with open(config["segments"]) as f:
        all_segments = json.load(f)

    segments = [s for s in all_segments if s.get("entry_count", 0) >= config.get("min_entries", 3)]

    if not segments:
        print("No segments match the criteria.", file=sys.stderr)
        return

    offset_ms = int(config.get("time_offset_seconds", 0) * 1000)
    outdir = Path(config["outdir"])
    outdir.mkdir(parents=True, exist_ok=True)

    unique = [s for s in segments if s.get("type") == "unique"]
    repeated = [s for s in segments if s.get("type") == "repeated"]

    if unique:
        print(f"Unique: {len(unique)} segments", file=sys.stderr)
        _output_category(outdir, "unique", unique, config.get("long_threshold", 10), offset_ms)
        html_path = outdir / "unique.html"
        html_path.write_text(_render_segments_html(unique, "Unique segments", offset_ms))
        print(f"         -> {html_path}", file=sys.stderr)

    if repeated:
        print(f"Repeated: {len(repeated)} segments", file=sys.stderr)
        _output_category(outdir, "repeated", repeated, config.get("long_threshold", 10), offset_ms)
        html_path = outdir / "repeated.html"
        html_path.write_text(_render_segments_html(repeated, "Repeated segments", offset_ms))
        print(f"           -> {html_path}", file=sys.stderr)
