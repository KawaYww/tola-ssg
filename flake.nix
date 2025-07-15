{
  description = "Description for the project";

  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
      ];
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" "mingw64" ];
      perSystem = { config, self', inputs', pkgs, system, ... }: {
        packages = {
          default = pkgs.callPackage ./default.nix { };
          static = pkgs.pkgsStatic.callPackage ./default.nix { };
          x86 = pkgs.pkgsCross.gnu64.callPackage ./default.nix { };
          x86-static = pkgs.pkgsCross.gnu64.pkgsStatic.callPackage ./default.nix { };
          aarch64 = pkgs.pkgsCross.aarch64-multiplatform.callPackage ./default.nix { };
          aarch64-static = pkgs.pkgsCross.aarch64-multiplatform.pkgsStatic.callPackage ./default.nix { };
          windows = pkgs.pkgsCross.mingwW64.callPackage ./default.nix { };
          darwin = pkgs.pkgsCross.aarch64-darwin.callPackage ./default.nix { };
        };
        
      };
      flake = {
      };
    };
}
