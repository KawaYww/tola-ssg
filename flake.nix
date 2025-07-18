{
  description = "A static site generator for typst-based blog, written in Rust";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [];
      systems = [ 
        "x86_64-linux" 
        "aarch64-linux" 
        "x86_64-darwin" 
        "aarch64-darwin" 
        "x86_64-windows"
      ];
      perSystem = { self', inputs', config, pkgs, lib, system, ... }:
        let
          overlayedPkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [ inputs.rust-overlay.overlays.default ];
          };
          rustStable = overlayedPkgs.rust-bin.stable.latest.minimal;
          buildPackage = pkgs': 
            pkgs'.rustPlatform.buildRustPackage rec {
              pname = "tola";
              version = "0.5.3";
              cargo = rustStable;
              rustc = rustStable;
              src = ./.;
              cargoLock.lockFile = src + /Cargo.lock;
              doCheck = false;
              meta = {
                description = "A static site generator for typst-based blog, written in Rust";
                homepage = "https://github.com/KawaYww/tola";
                license = lib.licenses.mit;
              };
            };
        in {
          packages = {
            default = buildPackage overlayedPkgs;
            static = buildPackage overlayedPkgs.pkgsStatic;

            x86 = buildPackage overlayedPkgs.pkgsCross.gnu64;
            x86-static = buildPackage overlayedPkgs.pkgsCross.gnu64.pkgsStatic;

            aarch64 = buildPackage overlayedPkgs.pkgsCross.aarch64-multiplatform;
            aarch64-static = buildPackage overlayedPkgs.pkgsCross.aarch64-multiplatform.pkgsStatic;

            windows = buildPackage overlayedPkgs.pkgsCross.mingwW64;
            darwin = buildPackage overlayedPkgs.pkgsCross.aarch64-darwin;
          };
        };
    };
}
