#!/bin/sh
# Full verification: the exact sequence CI runs on push/PR, and the exact
# sequence `scripts/dev.sh verify` runs locally. One image, one order.
#
#   scripts/verify.sh
set -eu

ROOT=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd)
cd "$ROOT" || exit 2

# One timestamped directory for every result this run produces. The children
# inherit LOG_NOTIFY_RUN_DIR, so they write into the same directory.
. scripts/run-dir.sh
printf '\n==> verify: results in %s\n' "$RUN_DIR"

# static all = rust (fmt, clippy+all tests in one pass, rustdoc) + repo lints
# + sast + deps (zero-tolerance advisories) + duplicates + sbom.
sh scripts/static-check.sh all

# Documentation lint (markdownlint-cli2, pinned in the image).
sh scripts/docs-check.sh

# Release build + static-link contract + tarball. The artifact is discarded:
# push builds never publish.
sh scripts/package-release.sh --check
