{
  description = "";
  inputs = {
    naersk.url = "github:nix-community/naersk/master";
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
      naersk,
      ...
    }:
    utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        naersk-lib = pkgs.callPackage naersk { };
        resha = naersk-lib.buildPackage ./.;
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
