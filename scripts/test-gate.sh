#!/bin/sh
# The test gate — single source of truth for scripts/dev.sh check and CI.
#
# SINGLE pass: clippy-driver compiles with the crate [lints] + CLIPPY_ARGS
# while building the tests, then runs them. No separate clippy build.
#
#   scripts/test-gate.sh
set -eu

# clippy-driver takes CLIPPY_ARGS as ONE argv: "-Dwarnings" (no space).
# RUST_MIN_STACK: libtest gives each test thread 2 MiB; the daemon stack needs
# more headroom in debug builds.
export RUSTC_WORKSPACE_WRAPPER=clippy-driver
export CLIPPY_ARGS=-Dwarnings
export RUST_MIN_STACK=16777216

cargo test --all-targets --locked
