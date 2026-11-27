# Pinned musl release artifact consumed by flake.nix.
#
# WHY a prebuilt tarball: nixpkgs does not cache musl toolchains; a source
# build would compile the musl toolchain for hours. The artifact is built in
# the Alpine CI container and pinned here.
#
# RELEASE CHECKLIST: CI updates version/url/sha256 on every `v*` tag; keep this
# as the manual fallback.
#   sha256sum log-notify-<ver>-x86_64-musl.tar.gz
{
  version = "0.1.0";
  url = "https://github.com/skea999/log-notify/releases/download/v0.1.0/log-notify-v0.1.0-x86_64-musl.tar.gz";
  # Replace with the release tarball sha256 before a manual flake build.
  sha256 = "2a225dce3052af5dd6fd8fe1489fcfcf3051c13e8f920a454b4d25a59d294c5e";
}
