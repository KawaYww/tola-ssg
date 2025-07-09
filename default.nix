{
  lib,
  rustPlatform,
  fetchFromGitHub,
  installShellFiles,
  stdenv,
}:

rustPlatform.buildRustPackage (finalAttrs: rec {
  pname = "tola";
  version = "0.4.10";

  src = ./.;
  
  # useFetchCargoVendor = true;
  # cargoHash = "sha256-bDQHdBxj4rarNnZDmPsClDaOqrdMCzM5usWX+9raYOU=";
  cargoLock.lockFile = src + /Cargo.lock;

  # There are not any tests in source project.
  doCheck = false;

  meta = {
    description = "A static site generator for typst-based blog, written in Rust";
    homepage = "https://github.com/KawaYww/tola";
    license = lib.licenses.mit;
  };
})
