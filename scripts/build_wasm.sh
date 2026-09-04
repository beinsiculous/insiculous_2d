#!/usr/bin/env bash
# Build a game's web (wasm) bundle in the site's drop-in layout.
#
# Usage: scripts/build_wasm.sh <game_dir> <slug> [--version vN] [--serve] [--sync <site_public_dir>]
#   game_dir   path to the game crate (e.g. ../games/pong)
#   slug       the site slug — output lands in dist/games/<slug>/<version>/
#   --version  bundle version dir, default v1. The version is a FOUR-place
#              contract (game src/web_entry.rs ASSET_BASE, this script's
#              output dir, the site's <slug>.md wasm: path, the deployed
#              public/games/<slug>/<version>/ dir); this script hard-fails
#              if the game's ASSET_BASE disagrees, so drift is loud.
#   --serve    serve dist/ on http://127.0.0.1:8080 after building
#   --sync     also copy the bundle into <site_public_dir>/games/<slug>/<version>
#              (e.g. ../insiculous_web/public). Refuses nothing — remember
#              the site rule: a version dir is immutable once DEPLOYED; only
#              sync over a version before its first live deploy, bump to the
#              next version after.
#
# Output (mirrors production URLs so the hardcoded asset base works both
# locally and deployed):
#   <game_dir>/dist/games/<slug>/<version>/{game.js, game_bg.wasm, assets/...}
#   <game_dir>/dist/games/<slug>/index.html   (local test page — NOT deployed)
set -euo pipefail

if [[ $# -lt 2 ]]; then
    echo "usage: $0 <game_dir> <slug> [--serve]" >&2
    exit 2
fi

GAME_DIR="$(cd "$1" && pwd)"
SLUG="$2"
shift 2
SERVE=""
SYNC_DIR=""
VERSION="v1"
while [[ $# -gt 0 ]]; do
    case "$1" in
        --serve)   SERVE="--serve"; shift ;;
        --sync)    SYNC_DIR="${2:?--sync needs a site public dir}"; shift 2 ;;
        --version) VERSION="${2:?--version needs a version dir like v2}"; shift 2 ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

# --- ASSET_BASE must agree with the version we are building ----------------
# The base URL is compiled INTO the wasm (web_entry.rs); a bundle built for
# one version dir but fetching assets from another 404s (or silently serves
# stale assets). Hard fail with the exact remediation.
WEB_ENTRY="$GAME_DIR/src/web_entry.rs"
if [[ -f "$WEB_ENTRY" ]]; then
    EXPECTED_BASE="/games/$SLUG/$VERSION/assets"
    if ! grep -q "\"$EXPECTED_BASE\"" "$WEB_ENTRY"; then
        ACTUAL=$(grep -o '"/games/[^"]*"' "$WEB_ENTRY" | head -1 || true)
        echo "ERROR: $WEB_ENTRY ASSET_BASE ($ACTUAL) != \"$EXPECTED_BASE\"." >&2
        echo "Fix:   set ASSET_BASE to \"$EXPECTED_BASE\" (the version is a 4-place contract)." >&2
        exit 1
    fi
fi

# --- wasm-bindgen CLI must match the crate version EXACTLY -----------------
# A mismatched CLI produces silently broken output, which is worse than a
# blocked build — hard fail, with the exact remediation.
LOCK_VERSION="$(awk '/^name = "wasm-bindgen"$/{getline; gsub(/"/,"",$3); print $3; exit}' "$GAME_DIR/Cargo.lock")"
CLI_VERSION="$(wasm-bindgen --version 2>/dev/null | awk '{print $2}' || true)"
if [[ -z "$LOCK_VERSION" ]]; then
    echo "ERROR: wasm-bindgen not in $GAME_DIR/Cargo.lock — is the wasm dep added?" >&2
    exit 1
fi
if [[ "$CLI_VERSION" != "$LOCK_VERSION" ]]; then
    echo "ERROR: wasm-bindgen CLI ($CLI_VERSION) != crate ($LOCK_VERSION)." >&2
    echo "Fix:   cargo install wasm-bindgen-cli --version $LOCK_VERSION --locked" >&2
    exit 1
fi

# --- build -----------------------------------------------------------------
if ! grep -q '^\[profile\.wasm-release\]' "$GAME_DIR/Cargo.toml"; then
    echo "ERROR: $GAME_DIR/Cargo.toml has no [profile.wasm-release] section." >&2
    echo "Add the wasm port boilerplate first (see ../games/pong/Cargo.toml:" >&2
    echo "[lib] cdylib+rlib, wasm-target deps, [profile.wasm-release])." >&2
    exit 1
fi
CRATE_NAME="$(awk -F'"' '/^name = /{print $2; exit}' "$GAME_DIR/Cargo.toml")"
WASM_FILE="$GAME_DIR/target/wasm32-unknown-unknown/wasm-release/${CRATE_NAME}.wasm"
OUT_DIR="$GAME_DIR/dist/games/$SLUG/$VERSION"

(cd "$GAME_DIR" && cargo build --lib --target wasm32-unknown-unknown --profile wasm-release)

rm -rf "$GAME_DIR/dist/games/$SLUG"
mkdir -p "$OUT_DIR"
wasm-bindgen --target web --no-typescript --out-name game --out-dir "$OUT_DIR" "$WASM_FILE"

# --- assets + manifest -----------------------------------------------------
# manifest.json lists every asset file relative to assets/; the web boot
# phase fetches each entry and stores it under {base}/{entry} (the canonical
# VFS key scheme).
mkdir -p "$OUT_DIR/assets"
if [[ -d "$GAME_DIR/assets" ]]; then
    cp -r "$GAME_DIR/assets/." "$OUT_DIR/assets/"
fi
(cd "$OUT_DIR/assets" && find . -type f ! -name manifest.json | sed 's|^\./||' | sort \
    | python3 -c 'import json,sys; print(json.dumps([l.strip() for l in sys.stdin if l.strip()], indent=2))' \
    > manifest.json)

# --- local test page (mirrors the site's GameEmbed contract) ---------------
read -r WIDTH HEIGHT <<< "$(python3 - "$GAME_DIR" <<'EOF'
import re, sys
# Best effort: pull WIN_W/WIN_H from the game's constants; fall back 800x600.
try:
    src = open(f"{sys.argv[1]}/src/constants.rs").read()
    w = re.search(r"WIN_W[^=\n]*=\s*([0-9]+)", src)
    h = re.search(r"WIN_H[^=\n]*=\s*([0-9]+)", src)
    print(int(w.group(1)) if w else 800, int(h.group(1)) if h else 600)
except OSError:
    print(800, 600)
EOF
)"

cat > "$GAME_DIR/dist/games/$SLUG/index.html" <<EOF
<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>$SLUG (wasm test page)</title>
<style>body{background:#111;color:#eee;font-family:monospace} canvas{outline:none}</style>
</head>
<body>
<p id="game-loading">Checking WebGPU…</p>
<canvas id="game-canvas" width="$WIDTH" height="$HEIGHT" tabindex="0"
        role="img" aria-label="$SLUG game canvas"></canvas>
<script type="module">
  const status = document.getElementById('game-loading');
  // Guard BEFORE the import, and mirror the site's GameEmbed gate: an adapter
  // alone is not proof, the engine asks for a device at the default limits.
  const adapter = navigator.gpu ? await navigator.gpu.requestAdapter().catch(() => null) : null;
  const device = adapter ? await adapter.requestDevice().catch(() => null) : null;
  device?.destroy();
  if (!adapter || !device) {
    status.textContent =
      'This game needs WebGPU. Use Chrome/Edge, or enable dom.webgpu.enabled in Firefox (full restart).';
  } else {
    status.textContent = 'Loading game…';
    try {
      const init = (await import('/games/$SLUG/$VERSION/game.js')).default;
      await init();
      document.getElementById('game-canvas').focus();
    } catch (e) {
      status.textContent = 'Failed to start: ' + e;
      throw e;
    }
  }
</script>
</body>
</html>
EOF

# --- size gate -------------------------------------------------------------
WASM_OUT="$OUT_DIR/game_bg.wasm"
SIZE_BYTES=$(stat -c%s "$WASM_OUT")
SIZE_MIB=$(python3 -c "print(f'{$SIZE_BYTES/1048576:.2f}')")
echo "wasm size: ${SIZE_MIB} MiB ($WASM_OUT)"
if (( SIZE_BYTES > 20 * 1048576 )); then
    echo "WARNING: over the 20 MiB gate (Cloudflare hard limit 25 MiB)." >&2
    echo "Levers: trim symphonia codecs, wasm-opt -Oz, brotli at the edge." >&2
fi

if [[ -n "$SYNC_DIR" ]]; then
    SYNC_TARGET="$SYNC_DIR/games/$SLUG/$VERSION"
    rm -rf "$SYNC_TARGET"
    mkdir -p "$(dirname "$SYNC_TARGET")"
    cp -r "$OUT_DIR" "$SYNC_TARGET"
    echo "synced bundle -> $SYNC_TARGET"
fi

if [[ "$SERVE" == "--serve" ]]; then
    echo "Serving http://127.0.0.1:8080/games/$SLUG/ (Ctrl-C to stop)"
    (cd "$GAME_DIR/dist" && python3 -m http.server 8080)
fi
