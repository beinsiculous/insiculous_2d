#!/usr/bin/env bash
# The audio provenance gate's own test: a tree where every audio file is named passes, and a
# tree with a stray file (no LICENSE at all) and an unnamed file (a LICENSE whose only cover
# for it is a glob) fails, naming exactly those two paths.
#
# Run from anywhere: scripts/tests/check_audio_provenance_test.sh
set -uo pipefail
cd "$(dirname "$0")/../.." || exit 2

gate=scripts/check_audio_provenance.sh
fixtures=scripts/tests/audio_provenance
failures=0

fail() {
    echo "FAIL  $1"
    failures=$((failures + 1))
}

if ! named_output=$("$gate" "$fixtures/named"); then
    fail "the fully named tree should pass: $named_output"
fi

unnamed_output=$("$gate" "$fixtures/unnamed")
unnamed_status=$?
[ "$unnamed_status" -eq 1 ] || fail "the tree with a stray file should exit 1, exited $unnamed_status"
grep -q "NO LICENSE  assets/sounds/stray.wav" <<< "$unnamed_output" \
    || fail "a file with no LICENSE above it should be reported: $unnamed_output"
grep -q "UNNAMED     crates/demo/fixtures/piece.mid" <<< "$unnamed_output" \
    || fail "a file covered only by a glob should be reported: $unnamed_output"
if grep -q "other.mid" <<< "$unnamed_output"; then
    fail "a named file should not be reported: $unnamed_output"
fi

if [ "$failures" -ne 0 ]; then
    echo "audio provenance test: $failures failed"
    exit 1
fi
echo "audio provenance test: passed"
