#!/usr/bin/env bash
set -euo pipefail

if [ $# -lt 1 ]; then
  echo "Usage: $0 <input.jsonl> [extra analyze flags...]" >&2
  echo "Example: $0 tmp/kmrb_2026-04-06.jsonl --min-count 5" >&2
  exit 1
fi

input="$1"
shift

basename=$(basename "$input" .jsonl)
outdir="tmp/${basename}"
mkdir -p "$outdir"

csv="$outdir/preprocessed.csv"
result="$outdir/result.json"
enriched="$outdir/${basename}_enriched.json"

echo "==> preprocess"
uv run repetition-analyzer preprocess "$input" --filter type=transcript --id-key id > "$csv"
echo "    $csv"

echo "==> analyze"
uv run repetition-analyzer analyze "$csv" --format json "$@" > "$result"
echo "    $result"

echo "==> enrich"
uv run repetition-analyzer enrich --source "$csv" --result "$result" > "$enriched"
echo "    $enriched"

echo "==> plot"
uv run repetition-analyzer plot "$enriched"
echo "    $outdir/repetitions.html"

echo "Done. Output in $outdir/"
