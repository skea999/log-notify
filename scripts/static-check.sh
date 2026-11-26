#!/bin/sh
# Canonical static analysis for log-notify. Run inside log-notify-ci:ci.
#   sh scripts/static-check.sh [rust|repo|sast|deps|duplicates|sbom|all]
set -u

ROOT=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd)
cd "$ROOT" || exit 2
. scripts/run-dir.sh
GROUP=${1:-all}
STATUS=0
mkdir -p "$RUN_DIR/static" "$RUN_DIR/duplicates" "$RUN_DIR/sbom"

step() {
  printf '\n==> %s\n' "$*"
}

run() {
  step "$*"
  if ! "$@"; then STATUS=1; fi
}

have() { command -v "$1" >/dev/null 2>&1; }

# A missing tool is a gate failure, never a silent skip.
need() {
  if ! have "$1"; then
    step "$1: MISSING — the gate cannot run; use log-notify-ci:ci"
    STATUS=1
    return 1
  fi
}

rust_group() {
  if ! cargo clippy --version >/dev/null 2>&1 || ! cargo fmt --version >/dev/null 2>&1; then
    step "rust lints: clippy/rustfmt missing — run in log-notify-ci:ci"
    STATUS=1
    return
  fi
  run cargo fmt --check
  # Lints ride the test gate (single pass) — same script as dev/CI.
  run sh scripts/test-gate.sh
  step "cargo doc"
  if ! RUSTDOCFLAGS="-D warnings" cargo doc --no-deps; then STATUS=1; fi
}

repo_group() {
  need typos && run typos
  if have editorconfig-checker; then
    run editorconfig-checker
  elif have ec; then
    run ec
  else
    step "editorconfig-checker: MISSING — the gate cannot run"
    STATUS=1
  fi
  set -- scripts/*.sh .githooks/pre-commit
  need shellcheck && run shellcheck -x "$@"
  need shfmt && run shfmt -d "$@"
  need taplo && run taplo fmt --check Cargo.toml rust-toolchain.toml deny.toml taplo.toml
  need actionlint && run actionlint
  need zizmor && run zizmor .github/workflows
}

sast_group() {
  need gitleaks && run gitleaks detect --source "$ROOT" --log-opts=--all --redact --no-banner
  if need cargo-geiger; then
    step "cargo-geiger report (compile-time unsafe_code=forbid is the gate)"
    if ! cargo geiger --all-features --output-format Json \
      >"$RUN_DIR/static/geiger-log-notify.json"; then STATUS=1; fi
  fi
}

deps_group() {
  run cargo deny check advisories licenses bans sources
  run cargo audit --deny warnings
  run cargo machete
}

duplicates_group() {
  run sh scripts/duplicates-gate.sh
}

sbom_group() {
  step "cargo cyclonedx"
  if cargo cyclonedx --format json --all --override-filename log-notify --quiet; then
    mv "$ROOT/log-notify.json" "$RUN_DIR/sbom/log-notify.cdx.json"
  else
    STATUS=1
  fi
}

case "$GROUP" in
rust) rust_group ;;
repo) repo_group ;;
sast) sast_group ;;
deps) deps_group ;;
duplicates) duplicates_group ;;
sbom) sbom_group ;;
all)
  rust_group
  repo_group
  sast_group
  deps_group
  duplicates_group
  sbom_group
  ;;
*)
  echo "unknown group: $GROUP" >&2
  exit 2
  ;;
esac

printf '\n==> static-check %s: %s\n' "$GROUP" "$([ "$STATUS" -eq 0 ] && echo PASS || echo FAIL)"
exit "$STATUS"
