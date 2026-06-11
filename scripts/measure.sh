#!/bin/bash
# Measure speed (wall time) and peak RAM (RSS) for textsift
# Usage: ./scripts/measure.sh <input.jsonl> [label]

set -e
INPUT="${1:-testdata/100k.jsonl}"
LABEL="${2:-baseline}"
TEXTSIFT="target/release/textsift"

echo "=== $LABEL ==="
echo "Input: $INPUT"

# Warmup
$TEXTSIFT "$INPUT" --field text > /dev/null 2>&1

# Single measured run with /usr/bin/time -l for both speed and RAM
/usr/bin/time -l $TEXTSIFT "$INPUT" --field text > /dev/null 2>/tmp/textsift_measure.txt

cat /tmp/textsift_measure.txt | head -5

echo ""
