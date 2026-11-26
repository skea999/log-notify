#!/bin/sh
# One entry point for local build/test/static/docs/release. Uses the same image
# and the same scripts as CI. The host never runs the Rust toolchain.
#
#   scripts/dev.sh image          build log-notify-ci:ci
#   scripts/dev.sh build          cargo build (release)
#   scripts/dev.sh test           full test gate (same command as CI)
#   scripts/dev.sh fmt            cargo fmt --check
#   scripts/dev.sh check          fmt + full test gate
#   scripts/dev.sh static [group] static analysis (default: all)
#   scripts/dev.sh docs           markdownlint
#   scripts/dev.sh verify         full sequence: static + docs + release
set -eu

ROOT=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd)
cd "$ROOT"
RUNTIME=${CONTAINER_RUNTIME:-podman}
CI_IMAGE=${LOG_NOTIFY_CI_IMAGE:-log-notify-ci:ci}
CARGO_HOME_DIR=${CARGO_HOME:-/work/artifacts/cargo-home}

run_ci() {
  set -- "$CI_IMAGE" "$@"
  [ -z "${RUN_IN:-}" ] || set -- -e "LOG_NOTIFY_RUN_DIR=/work/$RUN_IN" "$@"
  "$RUNTIME" run --rm --network=host -v "$ROOT:/work" -w /work \
    -e "CARGO_HOME=$CARGO_HOME_DIR" -e RUST_MIN_STACK=16777216 "$@"
}

# open_run_dir: create (and prune) the run directory, print its repo-relative
# path. Only commands that write results call it.
open_run_dir() {
  RUN_IN=$(sh scripts/run-dir.sh)
  printf 'results: %s\n' "$RUN_IN"
}

cmd=${1:-check}
shift || true
case "$cmd" in
image) "$RUNTIME" build -t "$CI_IMAGE" -f ci/Containerfile . ;;
build) run_ci cargo build --release ;;
test) run_ci sh scripts/test-gate.sh ;;
fmt) run_ci cargo fmt --check ;;
check)
  run_ci cargo fmt --check
  run_ci sh scripts/test-gate.sh
  ;;
static)
  open_run_dir
  run_ci sh scripts/static-check.sh "${1:-all}"
  ;;
docs) run_ci sh scripts/docs-check.sh ;;
verify)
  open_run_dir
  run_ci sh scripts/verify.sh
  ;;
*)
  echo "usage: $0 [image|build|test|fmt|check|static [group]|docs|verify]" >&2
  exit 2
  ;;
esac
