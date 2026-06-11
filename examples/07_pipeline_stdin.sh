#!/usr/bin/env bash
# CLI playbook: compose textsift with jq in a Unix pipeline.
#
# textsift reads from stdin (-) and writes to stdout, so it
# composes with other CLI tools.

set -euo pipefail

INPUT="${1:?Usage: $0 <input.jsonl>}"

TEXTSIFT="${TEXTSIFT:-$(command -v textsift 2>/dev/null || echo "./target/release/textsift")}"

echo "=== Filter + dedup + extract ==="
echo "Dedup, then extract the text field (first 5)"
echo

cat "$INPUT" \
  | "$TEXTSIFT" - --field text --stats \
  | jq -r '.text' \
  | head -5

echo
echo "=== Duplicate clusters (non-representatives) ==="
"$TEXTSIFT" "$INPUT" --field text --clusters \
  | jq -c 'select(.is_representative == false)' \
  | head -5
