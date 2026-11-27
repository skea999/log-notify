{
  description = "log-notify — tail logs and emit freedesktop notifications";

  # Stable nixpkgs: packaging only, no musl toolchain build.
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";

  outputs = {nixpkgs, ...}: let
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};
    bin = import ./nix/binary.nix;

    # Pinned musl release artifact. CI updates version/url/sha256 on tags.
    release = pkgs.stdenvNoCC.mkDerivation {
      pname = "log-notify-release";
      inherit (bin) version;

      src = pkgs.fetchurl {
        inherit (bin) url sha256;
      };

      nativeBuildInputs = [pkgs.gnutar];
      dontUnpack = true;
      installPhase = ''
        mkdir -p $out
        tar xzf $src -C $out
      '';
    };

    log-notify =
      pkgs.runCommand "log-notify-${bin.version}" {
        inherit (bin) version;
      } ''
        mkdir -p $out/bin $out/share/log-notify
        install -Dm0755 ${release}/bin/log-notify $out/bin/log-notify
        install -Dm0644 ${release}/config/log-notify.toml \
          $out/share/log-notify/log-notify.toml
        install -Dm0644 ${release}/README.md \
          $out/share/doc/log-notify/README.md
        install -Dm0644 ${release}/LICENSE \
          $out/share/doc/log-notify/LICENSE
      '';
  in {
    packages.${system} = {
      default = log-notify;
      inherit log-notify;
    };

    apps.${system}.default = {
      type = "app";
      program = "${log-notify}/bin/log-notify";
    };

    checks.${system}.package = log-notify;
  };
}
