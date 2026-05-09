#!/usr/bin/env bash
# check_corpus_drift.sh — guard the verbatim corpus block in
# tests/fixtures/corpus/array_define_real.pd against drift relative to
# array-define.txt.
#
# The first 319 `#X obj … array …;` entries of the fixture must, when stripped
# of their `#X obj X Y ` prefix and trailing `;`, equal the lines of
# array-define.txt byte-for-byte.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
FIXTURE="$REPO_ROOT/tests/fixtures/corpus/array_define_real.pd"
SOURCE="$REPO_ROOT/array-define.txt"

if [[ ! -f "$FIXTURE" ]]; then
    echo "missing fixture: $FIXTURE" >&2
    exit 1
fi
if [[ ! -f "$SOURCE" ]]; then
    echo "missing source: $SOURCE" >&2
    exit 1
fi

EXPECTED_LINES=$(wc -l < "$SOURCE")

# Extract the first $EXPECTED_LINES `#X obj` lines, strip prefix `#X obj X Y `
# and trailing `;`, leaving just the `array …` body.
DERIVED=$(grep -E '^#X obj [0-9]+ [0-9]+ array (define|d) ' "$FIXTURE" \
    | head -n "$EXPECTED_LINES" \
    | sed -E 's/^#X obj [0-9]+ [0-9]+ //; s/;$//')

if ! diff -u <(echo "$DERIVED") "$SOURCE" > /tmp/corpus_drift.diff; then
    echo "corpus drift detected — first 319 fixture rows do not match array-define.txt:" >&2
    head -40 /tmp/corpus_drift.diff >&2
    exit 1
fi

echo "corpus drift check OK ($EXPECTED_LINES rows match)"
