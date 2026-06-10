#!/usr/bin/env bash
#
# Cargo build shim invoked from src/meson.build.
#
# Meson's custom_target() runs a single program (it does not spawn a shell),
# so it cannot express "cargo build && cp <artifact> <output>" on its own.
# This wrapper bridges the two build systems: it runs Cargo inside Meson's
# build tree and copies the resulting binary to the path Meson expects.
#
# Arguments (positional, supplied by Meson):
#   $1  MESON_BUILD_ROOT   - absolute path to the Meson build directory
#   $2  MESON_SOURCE_ROOT  - absolute path to the project source root
#   $3  OUTPUT             - absolute path Meson wants the final binary at
#   $4  PROFILE            - "release" or "debug"
#   $5  APP_BIN            - the cargo binary name to copy

set -euo pipefail

MESON_BUILD_ROOT="$1"
MESON_SOURCE_ROOT="$2"
OUTPUT="$3"
PROFILE="$4"
APP_BIN="$5"

# Keep Cargo's artifacts and registry caches inside the Meson build tree so a
# `rm -rf _build` fully cleans the project and out-of-tree builds stay isolated.
export CARGO_TARGET_DIR="${MESON_BUILD_ROOT}/cargo-target"
export CARGO_HOME="${MESON_BUILD_ROOT}/cargo-home"

if [[ "$PROFILE" == "release" ]]; then
    echo "── cargo: building optimised release binary ──"
    cargo build \
        --manifest-path "${MESON_SOURCE_ROOT}/Cargo.toml" \
        --release
    cp "${CARGO_TARGET_DIR}/release/${APP_BIN}" "$OUTPUT"
else
    echo "── cargo: building debug binary ──"
    cargo build \
        --manifest-path "${MESON_SOURCE_ROOT}/Cargo.toml" \
        --verbose
    cp "${CARGO_TARGET_DIR}/debug/${APP_BIN}" "$OUTPUT"
fi
