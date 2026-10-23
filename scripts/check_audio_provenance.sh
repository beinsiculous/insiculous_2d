#!/usr/bin/env bash
# The audio provenance gate: every file under an `assets/music`, `assets/ai/music`,
# `assets/soundfonts` or `assets/sounds` directory, under the examples' own bank folder
# (`examples/soundfonts`, which sits under no `assets/` because the examples' asset base is
# `examples/`), or under a crate's top-level `fixtures/` directory
# (`crates/<crate>/fixtures/`), is named in a LICENSE in the Debian copyright format
# (https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/), as
# `crates/editor/assets/fonts/LICENSE` is. A composition and a given recording or MIDI file
# carry separate rights, so a file nobody accounted for is a file nobody may ship.
#
# The covering LICENSE is the nearest one walking up from the file's directory to the audio
# directory itself. "Named" means the file's path relative to that LICENSE's directory is a
# whole token of a `Files:` field (continuation lines included). Globs are not expanded on
# purpose: `Files: *` would wave through the stray file this gate exists to catch.
#
# Usage: scripts/check_audio_provenance.sh [root]   (default: the engine root)
# One line per offending path, exit 1 on any. `scripts/tests/` is never scanned: its
# fixtures break the rule deliberately, to prove the gate fails.
set -uo pipefail

if [ $# -gt 0 ]; then
    cd "$1" || exit 2
else
    cd "$(dirname "$0")/.." || exit 2
fi

# Prints every path token of every `Files:` field in the LICENSE given as $1, one per line.
license_file_names() {
    awk '
        /^Files:/ { in_files = 1; sub(/^Files:[ \t]*/, ""); print; next }
        in_files && /^[ \t]/ { print; next }
        { in_files = 0 }
    ' "$1" | tr -s ' \t' '\n\n' | sed '/^$/d'
}

misses=0
checked=0
while IFS= read -r audio_directory; do
    while IFS= read -r audio_file; do
        checked=$((checked + 1))
        license_directory=$(dirname "$audio_file")
        while [ ! -f "$license_directory/LICENSE" ] && [ "$license_directory" != "$audio_directory" ]; do
            license_directory=$(dirname "$license_directory")
        done
        if [ ! -f "$license_directory/LICENSE" ]; then
            echo "NO LICENSE  ${audio_file#./}  (none between it and ${audio_directory#./})"
            misses=$((misses + 1))
            continue
        fi
        relative_path=${audio_file#"$license_directory"/}
        if ! license_file_names "$license_directory/LICENSE" | grep -qxF "$relative_path"; then
            echo "UNNAMED     ${audio_file#./}  (not in a Files: field of ${license_directory#./}/LICENSE)"
            misses=$((misses + 1))
        fi
    done < <(find "$audio_directory" -type f ! -name LICENSE | sort)
done < <(
    find . \( -path ./.git -o -path ./scripts/tests -o -name target -o -name dist -o -name node_modules \) -prune \
        -o -type d \( -path '*/assets/music' -o -path '*/assets/ai/music' -o -path '*/assets/soundfonts' -o -path '*/assets/sounds' \
        -o -path './examples/soundfonts' -o -regex '\./crates/[^/]+/fixtures' \) -print | sort
)

if [ "$misses" -ne 0 ]; then
    echo "audio provenance gate: $misses of $checked audio files are not named in a LICENSE"
    exit 1
fi
echo "audio provenance gate: clean ($checked audio files named in a LICENSE)"
