#!/usr/bin/env bash
# Build a game's bank, <dir>/<stem>.sf2, and the LICENSE beside it: the FluidR3 GM presets
# its arrangement plays, cut from the full bank by scripts/subset_soundfont.py. The subset is
# committed in the game; each game's scripts/prepare_soundfont.sh is a wrapper over this
# script naming its stem and presets, and is how anyone rebuilds the bank byte for byte. The
# two sha256s in the LICENSE are its provenance.
#
# Usage: prepare_game_soundfont.sh [--bank path/to/FluidR3_GM.sf2] [--max-mib n]
#            <assets/soundfonts dir> <stem> <bank:program>...
#   Without --bank, Debian's fluid-soundfont-gm 3.1-5.3 package is fetched with
#   `apt download`; where apt cannot find that version or does not answer in five minutes,
#   the same package is fetched from snapshot.debian.org. Either way the bank must match the
#   sha256 below, or nothing is written. With --max-mib, a bank larger than n MiB is deleted,
#   no LICENSE is written, and the script exits 1.
#   One bank per directory: the LICENSE names the one bank beside it, and a second run into
#   the same directory replaces both.
set -euo pipefail

ENGINE_DIR="$(cd "$(dirname "$0")/.." && pwd)"
BANK_SHA256="74594e8f4250680adf590507a306655a299935343583256f3b722c48a1bc1cb0"
PACKAGE="fluid-soundfont-gm"
VERSION="3.1-5.3"
CURL_OPTIONS=(-fsSL --connect-timeout 20 --max-time 900)

usage() {
    echo "usage: $0 [--bank path/to/FluidR3_GM.sf2] [--max-mib n] <assets/soundfonts dir> <stem> <bank:program>..." >&2
    exit 2
}

fetch_from_snapshot() {
    local directory="$1"
    local hash
    hash="$(curl "${CURL_OPTIONS[@]}" "https://snapshot.debian.org/mr/binary/$PACKAGE/$VERSION/binfiles" \
        | python3 -c 'import json, sys; print(json.load(sys.stdin)["result"][0]["hash"])')"
    curl "${CURL_OPTIONS[@]}" -o "$directory/${PACKAGE}_${VERSION}_all.deb" "https://snapshot.debian.org/file/$hash"
}

BANK=""
MAX_MIB=""
while [[ $# -gt 0 && "$1" == --* ]]; do
    case "$1" in
        --bank)
            [[ $# -ge 2 ]] || usage
            BANK="$2"
            shift 2
            ;;
        --max-mib)
            [[ $# -ge 2 && "$2" =~ ^[0-9]+$ ]] || usage
            MAX_MIB="$2"
            shift 2
            ;;
        *)
            usage
            ;;
    esac
done
[[ $# -ge 3 ]] || usage
OUTPUT_DIR="$1"
STEM="$2"
shift 2
PRESETS=("$@")

if [[ -z "$BANK" ]]; then
    DOWNLOAD_DIR="$(mktemp -d)"
    trap 'rm -rf "$DOWNLOAD_DIR"' EXIT
    if ! (cd "$DOWNLOAD_DIR" && timeout 300 apt download "$PACKAGE=$VERSION" >/dev/null 2>&1); then
        echo "apt has no $PACKAGE $VERSION or did not answer; fetching it from snapshot.debian.org" >&2
        if ! fetch_from_snapshot "$DOWNLOAD_DIR"; then
            echo "ERROR: could not fetch $PACKAGE $VERSION. Offline, pass --bank with a FluidR3_GM.sf2 you have." >&2
            exit 1
        fi
    fi
    dpkg -x "$DOWNLOAD_DIR/${PACKAGE}_${VERSION}_all.deb" "$DOWNLOAD_DIR/unpacked"
    BANK="$DOWNLOAD_DIR/unpacked/usr/share/sounds/sf2/FluidR3_GM.sf2"
fi

if [[ "$(sha256sum "$BANK" | cut -d' ' -f1)" != "$BANK_SHA256" ]]; then
    echo "ERROR: $BANK is not FluidR3_GM.sf2 from $PACKAGE $VERSION (sha256 mismatch)." >&2
    exit 1
fi

mkdir -p "$OUTPUT_DIR"
OUTPUT="$OUTPUT_DIR/$STEM.sf2"
python3 "$ENGINE_DIR/scripts/subset_soundfont.py" "$BANK" "$OUTPUT" "${PRESETS[@]}"

if [[ -n "$MAX_MIB" ]]; then
    SIZE_BYTES="$(stat -c %s "$OUTPUT")"
    if (( SIZE_BYTES > MAX_MIB * 1024 * 1024 )); then
        rm -f "$OUTPUT"
        echo "ERROR: $STEM.sf2 is $SIZE_BYTES bytes, over the $MAX_MIB MiB cap; deleted, no LICENSE written." >&2
        exit 1
    fi
fi

cat > "$OUTPUT_DIR/LICENSE" <<EOF
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: FluidR3 GM (subset)
Upstream-Contact: Frank Wen <getfrank@gmail.com>
Source: Debian $PACKAGE $VERSION, FluidR3_GM.sf2
 sha256 $BANK_SHA256

Files: $STEM.sf2
Copyright: 2000-2002, 2008 Frank Wen
Comment: A subset of FluidR3_GM.sf2 made by scripts/subset_soundfont.py with the
 presets ${PRESETS[*]} (bank:program), their instruments and their samples,
 unedited. Re-running scripts/prepare_soundfont.sh reproduces it byte for byte.
 sha256 $(sha256sum "$OUTPUT" | cut -d' ' -f1)
License: MIT
 Permission is hereby granted, free of charge, to any person obtaining a copy of
 this software and associated documentation files (the "Software"), to deal in the
 Software without restriction, including without limitation the rights to use, copy,
 modify, merge, publish, distribute, sublicense, and/or sell copies of the Software,
 and to permit persons to whom the Software is furnished to do so, subject to the
 following conditions:
 .
 The above copyright notice and this permission notice shall be included in all
 copies or substantial portions of the Software.
 .
 THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED,
 INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
 PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT
 HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF
 CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
 OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
EOF
echo "$STEM.sf2 ready in $OUTPUT_DIR"
