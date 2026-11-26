#!/bin/sh
# Release package builder: the only way release binaries are produced.
# Builds a static musl binary, enforces the static-link contract, packs the
# tarball, and verifies the binary runs.
#
#   scripts/package-release.sh --check              # verify only, discard
#   scripts/package-release.sh --release NAME       # keep <run dir>/release/NAME
set -eu

ROOT=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd)
cd "$ROOT" || exit 2

MODE=${1:---check}
case "$MODE" in
--check) NAME=log-notify-latest-x86_64-musl.tar.gz ;;
--release)
  NAME=${2:?usage: $0 --release <tarball-name>}
  ;;
*)
  echo "usage: $0 [--check|--release <name>]" >&2
  exit 2
  ;;
esac

. scripts/run-dir.sh
RELEASE=$RUN_DIR/release
PKG=$RUN_DIR/pkg
mkdir -p "$RELEASE"

CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_RUSTFLAGS="-C target-feature=+crt-static" \
  cargo build --release --target x86_64-unknown-linux-musl --locked

BIN=artifacts/target/x86_64-unknown-linux-musl/release/log-notify
if readelf -d "$BIN" | grep -q NEEDED; then
  echo "error: log-notify is not static:" >&2
  readelf -d "$BIN" | grep NEEDED >&2
  exit 1
fi
echo "log-notify: static OK"

rm -rf "$PKG"
mkdir -p "$PKG/bin" "$PKG/config"
cp "$BIN" "$PKG/bin/"
cp config/log-notify.toml "$PKG/config/"
cp README.md LICENSE "$PKG/"

tar czf "$RELEASE/$NAME" -C "$PKG" .
(
  cd "$RELEASE"
  sha256sum "$NAME" >"$NAME.sha256"
)
tar tzf "$RELEASE/$NAME"
cat "$RELEASE/$NAME.sha256"

"$PKG/bin/log-notify" --version

if [ "$MODE" = "--check" ]; then
  echo "package: verified, artifact discarded (push builds are never published)"
  rm -f "$RELEASE/$NAME" "$RELEASE/$NAME.sha256"
else
  echo "package: $RELEASE/$NAME"
fi
