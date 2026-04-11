#!/usr/bin/env bash
set -euo pipefail

for f in tmp/kmrb_*.jsonl; do
    echo "========== $(basename "$f" .jsonl) =========="
    uv run repetition-analyzer pipeline "$f" --preset kmrb
    echo
done
