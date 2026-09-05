#!/usr/bin/env bash
# Build a game's or the playground's web (wasm) bundle in the site's drop-in layout.
#
# Usage: scripts/build_wasm.sh <crate_dir> <slug> [--kind games|playground] [--project <slug>=<title>=<dir>]... [--version vN] [--serve] [--sync <site_public_dir>]
#   crate_dir  path to the crate (e.g. ../games/pong or crates/playground)
#   slug       the site slug — for a game it names the output dir
#              dist/games/<slug>/<version>/; for the playground it names the
#              bundle in messages only (the output dir has no slug segment)
#   --kind     games (default) or playground
#   --project  playground only, repeatable: <slug>=<title>=<dir>. <dir>/assets
#              (relative to the caller's cwd) is copied to
#              assets/projects/<slug>/assets/ and listed in assets/projects.json
#              with a content hash. At least one is required.
#   --version  bundle version dir, default v1. The version is a FOUR-place
#              contract for a game (its src/web_entry.rs ASSET_BASE, this
#              script's output dir, the site's <slug>.md wasm: path, the
#              deployed public/games/<slug>/<version>/ dir) and a FIVE-place
#              one for the playground (ASSET_BASE, BUNDLE_VERSION, the output
#              dir, projects.json's bundle_version, the deployed
#              public/playground/<version>/ dir); this script hard-fails if
#              the crate's constants disagree, so drift is loud.
#   --serve    serve dist/ on http://127.0.0.1:8080 after building
#   --sync     also copy the bundle into <site_public_dir>/games/<slug>/<version>
#              or <site_public_dir>/playground/<version> (e.g.
#              ../insiculous_web/public). Refuses nothing — remember the site
#              rule: a version dir is immutable once DEPLOYED; only sync over a
#              version before its first live deploy, bump to the next version
#              after.
#
# Output (mirrors production URLs so the hardcoded asset base works both
# locally and deployed):
#   games:      <crate_dir>/dist/games/<slug>/<version>/{game.js, game_bg.wasm, assets/...}
#               <crate_dir>/dist/games/<slug>/index.html   (local test page — NOT deployed)
#   playground: <crate_dir>/dist/playground/<version>/{game.js, game_bg.wasm, assets/...}
#               <crate_dir>/dist/playground/index.html     (local test page — NOT deployed)
set -euo pipefail

if [[ $# -lt 2 ]]; then
    echo "usage: $0 <crate_dir> <slug> [--kind games|playground] [--project <slug>=<title>=<dir>]... [--version vN] [--serve] [--sync <site_public_dir>]" >&2
    exit 2
fi

GAME_DIR="$(cd "$1" && pwd)"
SLUG="$2"
shift 2

BUILD_KIND="games"
SERVE=""
SYNC_DIR=""
VERSION="v1"
PROJECT_DEFINITIONS=()

while [[ $# -gt 0 ]]; do
    case "$1" in
        --kind)    BUILD_KIND="${2:?--kind needs games or playground}"; shift 2 ;;
        --project) PROJECT_DEFINITIONS+=("${2:?--project needs <slug>=<title>=<dir>}"); shift 2 ;;
        --serve)   SERVE="--serve"; shift ;;
        --sync)    SYNC_DIR="${2:?--sync needs a site public dir}"; shift 2 ;;
        --version) VERSION="${2:?--version needs a version dir like v2}"; shift 2 ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

if [[ "$BUILD_KIND" != "games" && "$BUILD_KIND" != "playground" ]]; then
    echo "ERROR: --kind must be 'games' or 'playground'." >&2
    exit 2
fi
if [[ "$BUILD_KIND" == "playground" && ${#PROJECT_DEFINITIONS[@]} -eq 0 ]]; then
    echo "ERROR: --kind playground needs at least one --project <slug>=<title>=<dir>." >&2
    echo "       A deployed bundle with no projects cannot be fixed without a version bump and a redeploy, so the build refuses early." >&2
    exit 2
fi

# --- ASSET_BASE must agree with the version we are building ----------------
# The base URL is compiled INTO the wasm (web_entry.rs); a bundle built for
# one version dir but fetching assets from another 404s (or silently serves
# stale assets). The grep is anchored to the const declaration: the entry's
# header comment quotes the same string, and a check that matched the comment
# would pass on a stale header after a real bump. Hard fail with the exact
# remediation.
WEB_ENTRY="$GAME_DIR/src/web_entry.rs"
if [[ "$BUILD_KIND" == "playground" ]]; then
    EXPECTED_BASE="/playground/$VERSION/assets"
else
    EXPECTED_BASE="/games/$SLUG/$VERSION/assets"
fi
if [[ -f "$WEB_ENTRY" ]]; then
    if ! grep -q "const ASSET_BASE: &str = \"$EXPECTED_BASE\"" "$WEB_ENTRY"; then
        ACTUAL_BASE=$(grep -o '"/\(games\|playground\)/[^"]*"' "$WEB_ENTRY" | head -1 || true)
        echo "ERROR: $WEB_ENTRY ASSET_BASE ($ACTUAL_BASE) != \"$EXPECTED_BASE\"." >&2
        echo "Fix:   set ASSET_BASE to \"$EXPECTED_BASE\" (the version is a multi-place contract; see the header)." >&2
        exit 1
    fi
    if [[ "$BUILD_KIND" == "playground" ]] && ! grep -q "const BUNDLE_VERSION: &str = \"$VERSION\"" "$WEB_ENTRY"; then
        echo "ERROR: $WEB_ENTRY BUNDLE_VERSION != \"$VERSION\"." >&2
        echo "Fix:   set BUNDLE_VERSION to \"$VERSION\" (the playground's fifth place)." >&2
        exit 1
    fi
fi

# --- wasm-bindgen CLI must match the crate version EXACTLY -----------------
# A mismatched CLI produces silently broken output, which is worse than a
# blocked build — hard fail, with the exact remediation. The lockfile is the
# workspace's, not the crate's: a workspace member (the playground) has none
# of its own, and both queries name the manifest because this script never
# cds before them — a bare query run from the engine root would answer for the
# engine while building a game in $GAME_DIR.
ROOT_MANIFEST="$(cargo locate-project --manifest-path "$GAME_DIR/Cargo.toml" --workspace --message-format plain)"
WORKSPACE_ROOT="$(dirname "$ROOT_MANIFEST")"
LOCK_FILE="$WORKSPACE_ROOT/Cargo.lock"
LOCK_VERSION="$(awk '/^name = "wasm-bindgen"$/{getline; gsub(/"/,"",$3); print $3; exit}' "$LOCK_FILE")"
CLI_VERSION="$(wasm-bindgen --version 2>/dev/null | awk '{print $2}' || true)"
if [[ -z "$LOCK_VERSION" ]]; then
    echo "ERROR: wasm-bindgen not in $LOCK_FILE — is the wasm dep added?" >&2
    exit 1
fi
if [[ "$CLI_VERSION" != "$LOCK_VERSION" ]]; then
    echo "ERROR: wasm-bindgen CLI ($CLI_VERSION) != crate ($LOCK_VERSION)." >&2
    echo "Fix:   cargo install wasm-bindgen-cli --version $LOCK_VERSION --locked" >&2
    exit 1
fi

# --- build -----------------------------------------------------------------
if ! grep -q '^\[profile\.wasm-release\]' "$ROOT_MANIFEST"; then
    echo "ERROR: $ROOT_MANIFEST has no [profile.wasm-release] section." >&2
    echo "Add the wasm port boilerplate first (see ../games/pong/Cargo.toml:" >&2
    echo "[lib] cdylib+rlib, wasm-target deps, [profile.wasm-release])." >&2
    exit 1
fi

CRATE_NAME="$(awk -F'"' '/^name = /{print $2; exit}' "$GAME_DIR/Cargo.toml")"
TARGET_DIRECTORY="$(cargo metadata --manifest-path "$GAME_DIR/Cargo.toml" --no-deps --format-version 1 | python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])')"
WASM_FILE="$TARGET_DIRECTORY/wasm32-unknown-unknown/wasm-release/${CRATE_NAME}.wasm"

if [[ "$BUILD_KIND" == "playground" ]]; then
    DIST_BASE="$GAME_DIR/dist/playground"
    SYNC_SUBPATH="playground/$VERSION"
else
    DIST_BASE="$GAME_DIR/dist/games/$SLUG"
    SYNC_SUBPATH="games/$SLUG/$VERSION"
fi
OUT_DIR="$DIST_BASE/$VERSION"

(cd "$GAME_DIR" && cargo build --lib --target wasm32-unknown-unknown --profile wasm-release)

rm -rf "$DIST_BASE"
mkdir -p "$OUT_DIR"
wasm-bindgen --target web --no-typescript --out-name game --out-dir "$OUT_DIR" "$WASM_FILE"

# --- assets + manifest -----------------------------------------------------
# manifest.json lists every asset file relative to assets/; the web boot
# phase fetches each entry and stores it under {base}/{entry} (the canonical
# VFS key scheme).
mkdir -p "$OUT_DIR/assets"
if [[ "$BUILD_KIND" == "playground" ]]; then
    python3 - "$OUT_DIR/assets" "$VERSION" "${PROJECT_DEFINITIONS[@]}" <<'EOF'
import hashlib
import json
import os
import sys

output_assets_dir = sys.argv[1]
bundle_version = sys.argv[2]
project_definitions = sys.argv[3:]

manifests = []
for project_definition in project_definitions:
    parts = project_definition.split("=", 2)
    if len(parts) != 3:
        sys.stderr.write(f"ERROR: invalid --project spec: {project_definition}\n")
        sys.exit(1)
    project_slug, project_title, project_source_dir = parts
    assets_source_dir = os.path.join(project_source_dir, "assets")
    if not os.path.isdir(assets_source_dir):
        sys.stderr.write(f"ERROR: project assets directory not found: {assets_source_dir}\n")
        sys.exit(1)

    target_project_assets_dir = os.path.join(output_assets_dir, "projects", project_slug, "assets")
    os.makedirs(target_project_assets_dir, exist_ok=True)

    file_relative_paths = []
    for root_dir, _, filenames in os.walk(assets_source_dir):
        for filename in filenames:
            absolute_file_path = os.path.join(root_dir, filename)
            relative_path = os.path.relpath(absolute_file_path, assets_source_dir).replace("\\", "/")
            file_relative_paths.append(relative_path)
            target_file_path = os.path.join(target_project_assets_dir, relative_path)
            os.makedirs(os.path.dirname(target_file_path), exist_ok=True)
            with open(absolute_file_path, "rb") as source_file, open(target_file_path, "wb") as target_file:
                target_file.write(source_file.read())

    # The hash covers the sorted path list and the bytes, so a rename with
    # identical contents still reads as a changed bundle.
    file_relative_paths.sort()
    hasher = hashlib.sha256()
    for relative_path in file_relative_paths:
        hasher.update(relative_path.encode("utf-8"))
        hasher.update(b"\0")
        source_file_path = os.path.join(assets_source_dir, relative_path)
        with open(source_file_path, "rb") as file_handle:
            while chunk := file_handle.read(65536):
                hasher.update(chunk)

    manifests.append({
        "slug": project_slug,
        "title": project_title,
        "bundle_version": bundle_version,
        "content_hash": hasher.hexdigest(),
        "origin": "bundled",
    })

projects_json_path = os.path.join(output_assets_dir, "projects.json")
with open(projects_json_path, "w", encoding="utf-8") as projects_file:
    json.dump(manifests, projects_file, indent=2)
    projects_file.write("\n")
EOF
else
    if [[ -d "$GAME_DIR/assets" ]]; then
        cp -r "$GAME_DIR/assets/." "$OUT_DIR/assets/"
    fi
fi

# manifest.json is generated LAST so it lists projects.json and every copied
# file; a manifest written before the copies would leave boot fetching nothing.
# Only the top-level manifest is excluded: a project may carry its own
# assets/manifest.json (a game bundle copied in as a project), and that one
# must be preloaded like any other file.
(cd "$OUT_DIR/assets" && find . -type f ! -path ./manifest.json | sed 's|^\./||' | sort \
    | python3 -c 'import json,sys; print(json.dumps([l.strip() for l in sys.stdin if l.strip()], indent=2))' \
    > manifest.json)

# --- local test page (mirrors the site's embed contract; NOT deployed) ------
if [[ "$BUILD_KIND" == "playground" ]]; then
    PAGE_TITLE="playground"
    CANVAS_WIDTH=1280
    CANVAS_HEIGHT=800
    IMPORT_URL="/playground/$VERSION/game.js"
    # The site's PlaygroundEmbed provides this element; without it the
    # persistence banner is a silent no-op and the local check never sees it.
    BANNER_LINE='<p id="playground-banner" role="alert"></p>'
else
    PAGE_TITLE="$SLUG"
    read -r CANVAS_WIDTH CANVAS_HEIGHT <<< "$(python3 - "$GAME_DIR" <<'EOF'
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
    IMPORT_URL="/games/$SLUG/$VERSION/game.js"
    BANNER_LINE=""
fi

cat > "$DIST_BASE/index.html" <<EOF
<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>$PAGE_TITLE (wasm test page)</title>
<style>body{background:#111;color:#eee;font-family:monospace} canvas{outline:none}</style>
</head>
<body>
$BANNER_LINE
<p id="game-loading">Checking WebGPU…</p>
<canvas id="game-canvas" width="$CANVAS_WIDTH" height="$CANVAS_HEIGHT" tabindex="0"
        role="img" aria-label="$PAGE_TITLE canvas"></canvas>
<script type="module">
  const status = document.getElementById('game-loading');
  // Guard BEFORE the import, and mirror the site's embed gate: an adapter
  // alone is not proof, the engine asks for a device at the default limits.
  const adapter = navigator.gpu ? await navigator.gpu.requestAdapter().catch(() => null) : null;
  const device = adapter ? await adapter.requestDevice().catch(() => null) : null;
  device?.destroy();
  if (!adapter || !device) {
    status.textContent =
      'This needs WebGPU. Use Chrome/Edge, or enable dom.webgpu.enabled in Firefox (full restart).';
  } else {
    status.textContent = 'Loading…';
    try {
      const init = (await import('$IMPORT_URL')).default;
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
    SYNC_TARGET="$SYNC_DIR/$SYNC_SUBPATH"
    rm -rf "$SYNC_TARGET"
    mkdir -p "$(dirname "$SYNC_TARGET")"
    cp -r "$OUT_DIR" "$SYNC_TARGET"
    echo "synced bundle -> $SYNC_TARGET"
fi

if [[ "$SERVE" == "--serve" ]]; then
    echo "Serving http://127.0.0.1:8080/${SYNC_SUBPATH%/*}/ (Ctrl-C to stop)"
    (cd "$GAME_DIR/dist" && python3 -m http.server 8080)
fi
