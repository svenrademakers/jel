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
        target = "aarch64-unknown-linux-musl";
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            #(import ./merlin_gcc_overlay.nix)
            (import rust-overlay)
          ];
          crossSystem = {
            config = target;
            rustc.config = target;
          };
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
              "PREFIX="
            ];
            buildPhase = ''
              make $makeFlags install-utils 
            '';
            dontInstall = true;
          };

          ronaldo-streaming = rustPlatform.buildRustPackage {
            name = "ronaldo-streaming";
            version = "0.0.1";
            src = nix-filter.lib.filter {
              root = ./.;
              include = [
                ./Cargo.toml
                ./Cargo.lock
                ./hyper_rusttls
                ./ronaldos_config
                ./ronaldos_webserver
                ./uacme_renew
              ];
            };
            cargoLock = {
              lockFile = ./Cargo.lock;
            };
            nativeBuildInputs = [ rustToolchain ];
            outputs = [ "out" "www" ];
            installPhase = ''
              cp -r target $out
              cp -r ronaldos_webserver/www $www
            '';
          };

          default = stdenv.mkDerivation {
            name = "opkg-repository";
            version = "0.0.1";
            src = ./scripts;
            RUST_TARGET = target;
            nativeBuildInputs = [ rustToolchain ronaldo-streaming opkg-utils buildPackages.python39 ];
            installPhase = ''
              # workaround to call opkg scripts. They are loaded into the PATH
              # environment correctly, but the included shebangs cannot be
              # resolved by the nix environment.
              export OPKG_ROOT=${opkg-utils.out}/bin
              python3 create_ipk_packages.py -m ${ronaldo-streaming.src} -b ${ronaldo-streaming.out} $out ${ronaldo-streaming.www}
            '';
          };
        };

        devShells = {
          default = stdenv.mkDerivation rec {
            name = "native development shell";
            src = self;
            nativeBuildInputs = [ rustToolchain buildPackages.python39];
          };
        };
      }
    );
}
