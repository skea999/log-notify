#!/bin/sh
# Duplication gate.
#
# Source clones: jscpd with .jscpd.json (50 tokens, Rust + shell, threshold 0).
# Any clone fails the gate. Refactor the code, never raise the threshold.
#
# Dependency duplicates: `cargo tree -d` is written to the run directory as a
# report. Multiple versions are allowed, but the exact state is recorded.
set -eu

ROOT=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd)
cd "$ROOT" || exit 2
. scripts/run-dir.sh
DUP=$ROOT/$RUN_DIR/duplicates
REPORT=$DUP/dependency-duplicates.txt
mkdir -p "$DUP"

STATUS=0
step() { printf '\n==> %s\n' "$*"; }

step "source clones (jscpd, 50 tokens, threshold 0)"
if command -v jscpd >/dev/null 2>&1; then
  if ! jscpd --config "$ROOT/.jscpd.json" --output "$DUP/jscpd"; then
    echo "duplicates: source clones found — refactor them; never raise the threshold" >&2
    STATUS=1
  fi
else
  echo "duplicates: jscpd missing (run in log-notify-ci:ci)" >&2
  STATUS=1
fi

step "dependency duplicates (report)"
if cargo tree -d --locked >"$REPORT" 2>&1; then
  # Duplicate roots start at column 0 (`name vX.Y.Z`); children are indented
  # with box-drawing characters, so match the crate-name pattern only.
  count=$(grep -cE '^[a-z0-9_-]+ v[0-9]' "$REPORT" || true)
  echo "duplicates: $count duplicate crate versions in the dependency graph"
  echo "duplicates: report: $RUN_DIR/duplicates/dependency-duplicates.txt"
else
  echo "duplicates: cargo tree failed" >&2
  STATUS=1
fi

exit "$STATUS"
