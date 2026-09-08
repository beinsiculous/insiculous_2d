#!/usr/bin/env bash
# The wasm build gate (roadmap H8, issue #7): the whole workspace must keep
# compiling — and stay clippy-clean — for wasm32-unknown-unknown, so an
# engine change can't silently break the live web target. CI runs exactly
# this script (.github/workflows/wasm-check.yml), so the local command and
# the CI gate cannot drift.
# crates/playground targets wasm32 and depends on the editor crates, bringing them into this gate.
#
# Deliberately NO --all-targets, unlike the native clippy gate: dev-deps
# (ecs's criterion -> rayon, renderer's tokio) don't build on wasm and are
# not part of the shipped web bundle. build_wasm.sh builds with --lib for
# the same reason. Do not "fix" this asymmetry.
#
# Needs the target installed: rustup target add wasm32-unknown-unknown
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> cargo check --workspace --target wasm32-unknown-unknown"
cargo check --workspace --target wasm32-unknown-unknown

echo "==> cargo clippy --workspace --target wasm32-unknown-unknown -- -D warnings"
cargo clippy --workspace --target wasm32-unknown-unknown -- -D warnings

echo "wasm gate: clean"
