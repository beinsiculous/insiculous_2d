#!/usr/bin/env bash
# Build the spike's assets/ (gitignored): the fixtures it plays, a font, and the cast
# subset of FluidR3 GM. The subset is tens of megabytes and is reported, not committed,
# so this script is how anyone rebuilds it byte for byte.
#
# Usage: prepare_assets.sh [path/to/FluidR3_GM.sf2]
#   Without a path, Debian's fluid-soundfont-gm 3.1-5.3 package is fetched with
#   `apt download` into a temporary directory and unpacked with `dpkg -x`. Either way the
#   bank must match the sha256 below, or nothing is written.
set -euo pipefail

SPIKE_DIR="$(cd "$(dirname "$0")" && pwd)"
ENGINE_DIR="$(cd "$SPIKE_DIR/../../.." && pwd)"
FIXTURES="$ENGINE_DIR/crates/music_midi/fixtures"
ASSETS="$SPIKE_DIR/assets"
BANK_SHA256="74594e8f4250680adf590507a306655a299935343583256f3b722c48a1bc1cb0"
# The casting table's presets, as bank:program: piano, harpsichord, strings, trumpet,
# trombone, xylophone, glockenspiel, timpani, pizzicato, oboe, tuba, the standard kit.
CAST_PRESETS=(0:0 0:6 0:48 0:56 0:57 0:13 0:9 0:47 0:45 0:68 0:58 128:0)

if [[ $# -ge 1 ]]; then
    BANK="$1"
else
    DOWNLOAD_DIR="$(mktemp -d)"
    trap 'rm -rf "$DOWNLOAD_DIR"' EXIT
    (cd "$DOWNLOAD_DIR" && apt download fluid-soundfont-gm=3.1-5.3 >/dev/null)
    dpkg -x "$DOWNLOAD_DIR"/fluid-soundfont-gm_3.1-5.3_all.deb "$DOWNLOAD_DIR/unpacked"
    BANK="$DOWNLOAD_DIR/unpacked/usr/share/sounds/sf2/FluidR3_GM.sf2"
fi

if [[ "$(sha256sum "$BANK" | cut -d' ' -f1)" != "$BANK_SHA256" ]]; then
    echo "ERROR: $BANK is not FluidR3_GM.sf2 from fluid-soundfont-gm 3.1-5.3 (sha256 mismatch)." >&2
    exit 1
fi

rm -rf "$ASSETS"
mkdir -p "$ASSETS/music" "$ASSETS/sounds" "$ASSETS/soundfonts" "$ASSETS/fonts"
cp "$FIXTURES/bach-invention-02.mid" "$FIXTURES/dansenapolitaine-type0.mid" "$ASSETS/music/"
cp "$FIXTURES/sine_48000_mono.wav" "$ASSETS/sounds/"
cp "$FIXTURES/LICENSE" "$ASSETS/music/LICENSE"
cp "$FIXTURES/LICENSE" "$ASSETS/sounds/LICENSE"
cp "$ENGINE_DIR/crates/editor/assets/fonts/DejaVuSans.ttf" "$ASSETS/fonts/font.ttf"
cp "$ENGINE_DIR/crates/editor/assets/fonts/LICENSE" "$ASSETS/fonts/LICENSE"
python3 "$ENGINE_DIR/scripts/subset_soundfont.py" "$BANK" "$ASSETS/soundfonts/cast.sf2" "${CAST_PRESETS[@]}"

cat > "$ASSETS/soundfonts/LICENSE" <<EOF
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: FluidR3 GM (subset)
Upstream-Contact: Frank Wen <getfrank@gmail.com>
Source: Debian fluid-soundfont-gm 3.1-5.3, FluidR3_GM.sf2
 sha256 $BANK_SHA256

Files: cast.sf2
Copyright: 2000-2002, 2008 Frank Wen
Comment: A subset of FluidR3_GM.sf2 made by scripts/subset_soundfont.py with the
 presets ${CAST_PRESETS[*]} (bank:program), their instruments and their samples,
 unedited. Re-running prepare_assets.sh reproduces it byte for byte.
 sha256 $(sha256sum "$ASSETS/soundfonts/cast.sf2" | cut -d' ' -f1)
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
echo "assets ready in $ASSETS"
