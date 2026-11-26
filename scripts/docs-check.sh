#!/bin/sh
# Canonical documentation check. Run inside log-notify-ci:ci.
set -u

ROOT=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd)
cd "$ROOT" || exit 2

exec markdownlint-cli2
