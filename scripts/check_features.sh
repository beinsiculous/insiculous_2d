#!/usr/bin/env bash
# The no-physics gate: engine_core must keep compiling, and stay clippy-clean,
# with its default features off. The workspace gates never build that
# configuration, so a broken `cfg(not(feature = "physics"))` arm slips past
# `cargo check --workspace` until someone builds a physics-less game. It was
# green when the arms were written (the Sep 2026 cleanup) and nothing has
# checked it since; this script is the standing check, run on every batch.
#
# Deliberately `-p engine_core` alone: it is the one crate with a physics
# feature, and a workspace-wide --no-default-features would turn off features
# other crates need to build at all.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> cargo check -p engine_core --no-default-features --all-targets"
cargo check -p engine_core --no-default-features --all-targets

echo "==> cargo clippy -p engine_core --no-default-features --all-targets -- -D warnings"
cargo clippy -p engine_core --no-default-features --all-targets -- -D warnings

echo "features gate: clean"
