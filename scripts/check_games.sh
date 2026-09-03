#!/usr/bin/env bash
# Games verification gate: checks all six games with and without --features editor.
# Accepts optional --test flag to run `cargo test` instead of `cargo check`.
set -euo pipefail
cd "$(dirname "$0")/.."

CMD="check"
if [[ "${1:-}" == "--test" ]]; then
    CMD="test"
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
done

echo "All six games passed cargo $CMD + clippy (default + editor features)."
