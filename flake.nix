{
  description = "CLI tool to synchronize file generation tasks";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";

    src-block.url = "github:icetan/src-block";
    src-block.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    {
      self,
      nixpkgs,
      utils,
      ...
    }:
    utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        meta = pkgs.lib.importTOML ./Cargo.toml;
        resha = pkgs.pkgsStatic.rustPlatform.buildRustPackage {
          pname = meta.package.name;
          version = meta.package.version;
          src =
            with pkgs.lib.fileset;
            toSource {
              root = ./.;
              fileset = unions [
                ./Cargo.toml
                ./Cargo.lock
                ./src
              ];
            };
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
        };
      in
      {
        packages = {
          inherit resha;
          default = self.packages.${system}.resha;
        };
        devShells.default =
          with pkgs;
          mkShell {
            buildInputs = [
              cargo
              rustc
              rustfmt
              rustPackages.clippy
              rust-analyzer
            ];
            RUST_SRC_PATH = rustPlatform.rustLibSrc;
          };
      }
    )
    // {
      overlays.default = final: prev: { resha = self.packages.${prev.system}.resha; };
    };
}
