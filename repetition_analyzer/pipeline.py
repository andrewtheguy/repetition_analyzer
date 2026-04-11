"""Run the full analysis pipeline: preprocess → (clip) → analyze → enrich → plot."""

import csv
import io
import sys
import os
from typing import Any


def _clip_after_ms(rows: list[dict[str, str]], cutoff_ms: int) -> list[dict[str, str]]:
    return [r for r in rows if int(r["start_ms"]) < cutoff_ms]


def _clip_at_rebroadcast_marker(
    rows: list[dict[str, str]], marker: str, tail_search: int = 60,
) -> list[dict[str, str]]:
    """Find the last occurrence of marker within the final tail_search entries and drop from there."""
    start = max(0, len(rows) - tail_search)
    cut = None
    for i in range(len(rows) - 1, start - 1, -1):
        if marker in rows[i]["text"]:
            cut = i
            break
    if cut is not None:
        dropped = len(rows) - cut
        print(f"  Found rebroadcast marker at row {cut} ({rows[cut]['start_formatted']}), dropping {dropped} trailing entries", file=sys.stderr)
        return rows[:cut]
    print(f"  Warning: rebroadcast marker '{marker}' not found in last {tail_search} entries, using time-based clip only", file=sys.stderr)
    return rows


def _preprocess_to_rows(config: dict[str, Any]) -> list[dict[str, str]]:
    """Run preprocess and capture output as rows."""
    from .preprocess import run_preprocess

    buf = io.StringIO()
    old_stdout = sys.stdout
    sys.stdout = buf
    try:
        run_preprocess(config)
    finally:
        sys.stdout = old_stdout

    buf.seek(0)
    reader = csv.DictReader(buf)
    return list(reader)


def _write_csv(rows: list[dict[str, str]], path: str, fieldnames: list[str]) -> None:
    with open(path, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(rows)


_CSV_FIELDS = ["id", "text", "start_ms", "end_ms", "start_formatted", "end_formatted"]


def run_pipeline(config: dict[str, Any]) -> None:
    input_path: str = config["file"]
    preset: str | None = config.get("preset")
    extra_analyze: list[str] = config.get("extra_analyze_args", [])

    basename = os.path.splitext(os.path.basename(input_path))[0]
    outdir = os.path.join("tmp", basename)
    os.makedirs(outdir, exist_ok=True)

    csv_path = os.path.join(outdir, "preprocessed.csv")
    result_path = os.path.join(outdir, "result.json")
    enriched_path = os.path.join(outdir, f"{basename}_enriched.json")

    # -- preprocess --
    print("==> preprocess", file=sys.stderr)
    preprocess_config: dict[str, Any] = {
        "file": input_path,
        "text_key": "text",
        "id_key": None,
        "start_ms_key": "start_ms",
        "end_ms_key": "end_ms",
        "start_formatted_key": "start_formatted",
        "end_formatted_key": "end_formatted",
        "filter": None,
    }

    if preset == "kmrb":
        preprocess_config["filter"] = "type=transcript"
        preprocess_config["id_key"] = "id"

    rows = _preprocess_to_rows(preprocess_config)
    total = len(rows)
    print(f"  {total} entries", file=sys.stderr)

    # -- clip (preset-specific) --
    if preset == "kmrb":
        print("==> clip (kmrb: drop rebroadcasts after 16:00:00)", file=sys.stderr)
        rows = _clip_after_ms(rows, 16 * 3600 * 1000)
        rows = _clip_at_rebroadcast_marker(rows, "敬請留意")
        print(f"  {len(rows)} entries after clipping (dropped {total - len(rows)})", file=sys.stderr)

    _write_csv(rows, csv_path, _CSV_FIELDS)
    print(f"  {csv_path}", file=sys.stderr)

    # -- analyze --
    print("==> analyze", file=sys.stderr)
    from .analyze import run_analyze

    analyze_config: dict[str, Any] = {"file": csv_path, "format": "json"}
    # Parse extra flags like --min-count 5
    it = iter(extra_analyze)
    for flag in it:
        key = flag.lstrip("-").replace("-", "_")
        val: Any = next(it, None)
        if val is not None:
            for cast in (int, float):
                try:
                    val = cast(val)
                    break
                except ValueError:
                    pass
        analyze_config[key] = val

    old_stdout = sys.stdout
    with open(result_path, "w") as f:
        sys.stdout = f
        try:
            run_analyze(analyze_config)
        finally:
            sys.stdout = old_stdout
    print(f"  {result_path}", file=sys.stderr)

    # -- enrich --
    print("==> enrich", file=sys.stderr)
    from .enrich import run_enrich

    with open(enriched_path, "w") as f:
        sys.stdout = f
        try:
            run_enrich({"source": csv_path, "result": result_path})
        finally:
            sys.stdout = old_stdout
    print(f"  {enriched_path}", file=sys.stderr)

    # -- plot --
    print("==> plot", file=sys.stderr)
    from .plot import run_plot

    run_plot(enriched_path)
    print(f"  {outdir}/", file=sys.stderr)

    print(f"Done. Output in {outdir}/", file=sys.stderr)
