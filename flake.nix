{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    utils.url = "github:numtide/flake-utils";
  };
  outputs =
    { nixpkgs, utils, ... }:
    utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            pkgs.cmake
            pkgs.rustc
            pkgs.libpq
            pkgs.openssl
            pkgs.pkg-config
            pkgs.protobuf
            pkgs.rust-jemalloc-sys
            pkgs.libgcc
            pkgs.postgresql
            pkgs.unzip
            pkgs.wget
          ];
        };
      }
    );
}
