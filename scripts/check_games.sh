#!/usr/bin/env bash
# Games verification gate: checks all six games with and without --features editor.
# Accepts optional --test flag to run `cargo test` instead of `cargo check`.
#
# Each game is also checked for the wasm32 target, with and without the feature:
# src/web_entry.rs is cfg(target_arch = "wasm32"), so no native check ever compiles
# it, and both branches now ship — the plain one as the game's bundle, the feature
# one as its editor bundle. Needs the wasm target installed (asserted below).
set -euo pipefail
cd "$(dirname "$0")/.."

CMD="check"
if [[ "${1:-}" == "--test" ]]; then
    CMD="test"
fi

if ! rustup target list --installed | grep -q wasm32-unknown-unknown; then
    echo "ERROR: the wasm32-unknown-unknown target is not installed." >&2
    echo "Fix:   rustup target add wasm32-unknown-unknown" >&2
    exit 1
fi

GAMES=(pong snake breakout frogger asteroids space_invaders)

for game in "${GAMES[@]}"; do
    echo "==> cargo $CMD -p $game (default)"
    cargo $CMD --manifest-path "../games/$game/Cargo.toml"
    echo "==> cargo clippy -p $game (default)"
    cargo clippy --manifest-path "../games/$game/Cargo.toml" --all-targets
    echo "==> cargo $CMD -p $game (--features editor)"
    cargo $CMD --manifest-path "../games/$game/Cargo.toml" --features editor
    echo "==> cargo clippy -p $game (--features editor)"
    cargo clippy --manifest-path "../games/$game/Cargo.toml" --features editor --all-targets
    echo "==> cargo check -p $game --lib --target wasm32-unknown-unknown (default)"
    cargo check --manifest-path "../games/$game/Cargo.toml" --lib --target wasm32-unknown-unknown
    echo "==> cargo check -p $game --lib --target wasm32-unknown-unknown (--features editor)"
    cargo check --manifest-path "../games/$game/Cargo.toml" --lib --target wasm32-unknown-unknown --features editor
done

echo "All six games passed cargo $CMD + clippy (default + editor features) and both wasm32 --lib checks."
