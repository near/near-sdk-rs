# This is a simple deterministic rust development environment
# This exposes Cargo, rustfmt, rust-analyzer and clippy
# This does not allow you to build binaries using nix
{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };
  outputs = { self, nixpkgs, flake-utils, rust-overlay, ... }:

    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        # Pick what rust compiler to use
        rustVersion = pkgs.rust-bin.stable.latest.default;
      in {
        devShell = pkgs.mkShell {

          # Everything in this list is added to your path
          buildInputs =
            with pkgs; [
              curl
              darwin.apple_sdk.frameworks.Security
              # pkgconfig
              # openssl
            ];
        };
      });
}
