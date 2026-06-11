#!/usr/bin/env bash
# CLI playbook: dedup a large JSONL file and measure the results.
#
# Usage:
#   chmod +x examples/06_large_jsonl_cli.sh
#   ./examples/06_large_jsonl_cli.sh data.jsonl text

set -euo pipefail

INPUT="${1:?Usage: $0 <input.jsonl> <field>}"
FIELD="${2:?Usage: $0 <input.jsonl> <field>}"

TEXTSIFT="${TEXTSIFT:-$(command -v textsift 2>/dev/null || echo "./target/release/textsift")}"

BEFORE=$(wc -l < "$INPUT" | tr -d ' ')
echo "Input: $INPUT ($BEFORE lines)"
echo "Field: $FIELD"
echo

# Step 1: Check how many duplicates exist
echo "=== Stats ==="
"$TEXTSIFT" "$INPUT" --field "$FIELD" --stats > /dev/null
echo

# Step 2: Write clean output
OUTPUT="${INPUT%.jsonl}_clean.jsonl"
echo "=== Dedup ==="
time "$TEXTSIFT" "$INPUT" --field "$FIELD" -o "$OUTPUT"

AFTER=$(wc -l < "$OUTPUT" | tr -d ' ')
REMOVED=$((BEFORE - AFTER))
echo
echo "Result: $BEFORE → $AFTER docs ($REMOVED removed)"
echo "Clean file: $OUTPUT"
