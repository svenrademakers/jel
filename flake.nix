{
  description = "flake for building ronaldo streaming";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    nix-filter.url = "github:numtide/nix-filter";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, nix-filter }:
    flake-utils.lib.eachSystem [ "x86_64-linux" ] (system:
      let
        target = "aarch64-unknown-linux-gnu";
        #target = "aarch64-unknown-linux-musl";
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            (import rust-overlay)
          ];
          crossSystem = { config = target;
          rustc.config = target;};
        };
        rustToolchain = (pkgs.buildPackages.rust-bin.stable.latest.default.override {
          targets = [ target ];
        });
      in
      with pkgs;
      {
        packages = rec{
          opkg-utils = stdenv.mkDerivation {
            name = "opkg-utils";
            version = "1.0.0";
            src = fetchgit {
              url =
                "https://git.yoctoproject.org/opkg-utils";
              sha256 = "kO4mUJKE6vtiOIvCiMcYo+5UuoL8AzpmT1hluHrlafg=";
            };
            makeFlags = [
              "DESTDIR=${placeholder "out"}"
              "PREFIX=/"
            ];
            buildPhase = ''
              make $makeFlags install-utils 
            '';
            dontInstall = true;
          };

          default = rustPlatform.buildRustPackage {
            pname = "ronaldos-opkg-repository";
            version = "0.0.1";
            src = ./.;

            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            nativeBuildInputs = [ rustToolchain opkg-utils buildPackages.python39 ];
            outputs = [ "out" "www" ];
            installPhase = ''
              python3 scripts/create_ipk_packages.py ronaldos_repository
              cp -r ronaldos_webserver/www $www
              cp -r target $out
            '';
          };
        };
      }
    );
}
