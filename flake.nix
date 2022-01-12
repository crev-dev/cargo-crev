{
  description = "Auction Sniper in Rust";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";

    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, naersk, nixpkgs, flake-utils, flake-compat, fenix }:
    flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = nixpkgs.legacyPackages."${system}";
      fenix-pkgs = fenix.packages.${system};
      fenix-channel = fenix-pkgs.complete;
      naersk-lib = naersk.lib."${system}".override {
        inherit (fenix-pkgs.minimal) cargo rustc;
      };
    in rec {
      packages.cargo-crev = naersk-lib.buildPackage {
        name = "cargo-crev";
        version = "0.21.4";
        src = ./.;
        targets = [ "cargo-crev" ];
        buildInputs = [ pkgs.openssl pkgs.perl ];
      };

      defaultPackage = self.packages.${system}.cargo-crev;
      defaultApp = self.packages.${system}.cargo-crev;

      # `nix develop`
      devShell = pkgs.mkShell
        {
          inputsFrom = builtins.attrValues self.packages.${system};
          buildInputs = [ pkgs.openssl pkgs.perl ];
          nativeBuildInputs = (with pkgs;
            [
              pkgconfig
              fenix-pkgs.rust-analyzer
              fenix-channel.rustfmt
              fenix-channel.rustc
            ]);
          RUST_SRC_PATH = "${fenix-channel.rust-src}/lib/rustlib/src/rust/library";
        };
  });
}
