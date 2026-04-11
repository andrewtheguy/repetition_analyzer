"""Plot repetition counts from enriched JSON as an HTML bar chart."""

import html
import json

TRUNCATE_LEN = 40

COLORS = {
    "Exact duplicate": "#e74c3c",
    "Near duplicate": "#f39c12",
    "Repeated n-gram": "#3498db",
    "Repeated sequence": "#2ecc71",
}


def truncate(text, max_len=TRUNCATE_LEN):
    return text[:max_len] + "\u2026" if len(text) > max_len else text


def collect_items(data):
    items = []

    for dup in data.get("exact_duplicates", []):
        details = [
            {"text": dup["canonical_text"], "start": ts.get("start_formatted", ""), "end": ts.get("end_formatted", "")}
            for ts in dup.get("index_timestamps", [])
        ]
        items.append((dup["canonical_text"], dup["count"], "Exact duplicate", details))

    for cluster in data.get("near_duplicates", []):
        details = [
            {"text": ts.get("text", ""), "start": ts.get("start_formatted", ""), "end": ts.get("end_formatted", "")}
            for ts in cluster.get("index_timestamps", [])
        ]
        items.append((cluster["representative_text"], len(cluster.get("members", [])), "Near duplicate", details))

    for ng in data.get("ngrams", []):
        details = [
            {"text": ng["ngram"], "start": ts.get("start_formatted", ""), "end": ts.get("end_formatted", "")}
            for ts in ng.get("index_timestamps", [])
        ]
        items.append((ng["ngram"], ng["count"], "Repeated n-gram", details))

    for seq in data.get("repeated_sequences", []):
        text = " / ".join(seq.get("entry_texts", [f"Sequence (len {seq.get('length', '?')})"]))
        details = [
            {"text": text, "start": occ.get("start_formatted", ""), "end": occ.get("end_formatted", "")}
            for occ in seq.get("occurrences", [])
        ]
        items.append((text, len(seq.get("occurrences", [])), "Repeated sequence", details))

    items.sort(key=lambda x: x[1], reverse=True)
    return items


def render_detail_rows(details):
    rows = ""
    for d in details:
        text = html.escape(d["text"])
        time = ""
        if d["start"] and d["end"]:
            time = f'{html.escape(d["start"])} \u2013 {html.escape(d["end"])}'
        rows += f'<div class="detail-row"><span class="detail-time">{time}</span><span class="detail-text">{text}</span></div>\n'
    return rows


def render_html(items):
    max_count = max(c for _, c, _, _ in items)
    categories_present = sorted(set(cat for _, _, cat, _ in items))

    legend = "".join(
        f'<span style="display:inline-flex;align-items:center;margin-right:1.5em;">'
        f'<span style="width:14px;height:14px;background:{COLORS[c]};border-radius:3px;display:inline-block;margin-right:6px;"></span>'
        f'{html.escape(c)}</span>'
        for c in categories_present
    )

    rows = ""
    for i, (text, count, category, details) in enumerate(items):
        pct = count / max_count * 100
        color = COLORS[category]
        label = html.escape(truncate(text))
        detail_html = render_detail_rows(details)
        rows += f"""<tr class="main-row" onclick="toggle({i})">
  <td class="label">{label}</td>
  <td class="count">{count}</td>
  <td class="bar-cell"><div class="bar" style="width:{pct:.1f}%;background:{color};"></div></td>
</tr>
<tr class="detail-panel" id="detail-{i}">
  <td colspan="3"><div class="detail-content">{detail_html}</div></td>
</tr>
"""

    return f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Repetitions</title>
<style>
  body {{ font-family: system-ui, -apple-system, sans-serif; margin: 2em; background: #fafafa; }}
  h1 {{ font-size: 1.4em; margin-bottom: 0.3em; }}
  .legend {{ margin-bottom: 1.5em; font-size: 0.9em; }}
  table {{ border-collapse: collapse; width: 100%; }}
  td {{ padding: 4px 8px; vertical-align: middle; }}
  .label {{ white-space: nowrap; text-align: right; width: 1%; font-size: 0.9em; }}
  .count {{ text-align: right; width: 1%; font-weight: bold; font-size: 0.9em; padding-right: 12px; }}
  .bar-cell {{ width: 98%; }}
  .bar {{ height: 22px; border-radius: 3px; min-width: 2px; }}
  .main-row {{ cursor: pointer; }}
  .main-row:hover {{ background: #eee; }}
  .detail-panel {{ display: none; }}
  .detail-panel.open {{ display: table-row; }}
  .detail-content {{ padding: 8px 12px; background: #f0f0f0; border-radius: 4px; margin: 4px 0; }}
  .detail-row {{ padding: 3px 0; display: flex; gap: 12px; font-size: 0.85em; }}
  .detail-time {{ color: #666; white-space: nowrap; min-width: 180px; }}
  .detail-text {{ flex: 1; }}
</style>
</head>
<body>
<h1>Repetitions</h1>
<div class="legend">{legend}</div>
<table>
{rows}
</table>
<script>
function toggle(i) {{
  document.getElementById('detail-' + i).classList.toggle('open');
}}
</script>
</body>
</html>"""


def run_plot(path: str) -> None:
    with open(path) as f:
        data = json.load(f)

    items = collect_items(data)
    if not items:
        print("No repetitions found.")
        return

    out = path.replace("_enriched.json", "_repetitions.html")
    with open(out, "w") as f:
        f.write(render_html(items))
    print(f"Saved to {out}")
