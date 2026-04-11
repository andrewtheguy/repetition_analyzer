#!/usr/bin/env bash
set -euo pipefail

for f in tmp/kmrb_*.jsonl; do
    echo "========== $(basename "$f" .jsonl) =========="
    # 7am
    uv run repetition-analyzer pipeline "$f" --preset kmrb --time-offset-seconds 25200
    echo
done
