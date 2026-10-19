#!/usr/bin/env bash
# The guide-table gate: every crate guide's "Pitfalls and their guard tests"
# table cites the test that fails when a trap regresses, as
# `src/file.rs test_name` or `tests/file.rs test_name` (relative to the crate
# root), or `— none` when the trap is unguarded. Nothing else verifies the
# citations, so the first rename orphans a row silently and the next reader
# concludes the guard was deleted. This script resolves every citation: the
# file must exist and declare `fn test_name`. One line per miss, exit 1 on any.
#
# Run from the engine root; `/finish-task` Gate 3 and every batch gate list it.
set -uo pipefail
cd "$(dirname "$0")/.."

misses=0
checked=0
for guide in crates/*/CLAUDE.md; do
    crate_dir=$(dirname "$guide")
    while read -r file test; do
        [ -n "$file" ] || continue
        checked=$((checked + 1))
        if [ ! -f "$crate_dir/$file" ]; then
            echo "MISSING FILE  $crate_dir/$file  (cited for $test in $guide)"
            misses=$((misses + 1))
        elif ! grep -qE "fn $test\b" "$crate_dir/$file"; then
            echo "MISSING TEST  $crate_dir/$file  $test  (cited in $guide)"
            misses=$((misses + 1))
        fi
    done < <(
        grep -E '^\|' "$guide" \
            | awk -F'|' 'NF >= 4 { print $3 }' \
            | grep -oE '`(src|tests)/[A-Za-z0-9_/.]+ [A-Za-z0-9_]+`' \
            | tr -d '`'
    )
done

if [ "$misses" -ne 0 ]; then
    echo "guide tests gate: $misses of $checked citations do not resolve"
    exit 1
fi
echo "guide tests gate: clean ($checked citations resolve)"
