{
  description = "Cryptographically verifiable Code REviews";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    flake-utils.url = "github:numtide/flake-utils";
    flakebox.url = "github:rustshop/flakebox";
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      flakebox,
      flake-compat,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        lib = pkgs.lib;
        projectName = "cargo-crev";

        flakeboxLib = flakebox.lib.mkLib pkgs {
          config = {
            github.ci.buildOutputs = [ ".#ci.${projectName}" ];
            just.importPaths = [ "justfile.custom.just" ];
            just.rules.watch.enable = false;
          };
        };

        # All paths needed for building the workspace
        buildPaths = [
          "Cargo.toml"
          "Cargo.lock"
          "cargo-crev"
          "crev-common"
          "crev-data"
          "crev-lib"
          "crev-wot"
        ];

        buildSrc = flakeboxLib.filterSubPaths {
          root = builtins.path {
            name = projectName;
            path = ./.;
          };
          paths = buildPaths;
        };

        multiBuild = (flakeboxLib.craneMultiBuild { }) (
          craneLib':
          let
            craneLib = craneLib'.overrideArgs {
              pname = projectName;
              src = buildSrc;
              nativeBuildInputs = with pkgs; [
                pkg-config
                perl
              ];
              buildInputs =
                with pkgs;
                [
                  openssl
                ]
                ++ lib.optionals stdenv.isDarwin [
                  libiconv
                  curl
                  libgit2
                  darwin.apple_sdk.frameworks.Security
                  darwin.apple_sdk.frameworks.CoreFoundation
                ];
              LIBCLANG_PATH = "${pkgs.libclang.lib}/lib/";
            };
          in
          rec {
            workspaceDeps = craneLib.buildWorkspaceDepsOnly { };

            workspace = craneLib.buildWorkspace {
              cargoArtifacts = workspaceDeps;
            };

            tests = craneLib.cargoNextest {
              cargoArtifacts = workspace;
            };

            clippy = craneLib.cargoClippy {
              cargoArtifacts = workspaceDeps;
            };

            cargo-crev = craneLib.buildPackage {
              cargoArtifacts = workspaceDeps;
              cargoExtraArgs = "--bin cargo-crev";
            };
          }
        );

        cargo-crev-container = pkgs.dockerTools.buildLayeredImage {
          name = projectName;
          contents = [ multiBuild.cargo-crev ];
          config = {
            Cmd = [ "${multiBuild.cargo-crev}/bin/cargo-crev" ];
          };
        };
      in
      {
        packages = {
          default = multiBuild.cargo-crev;
          cargo-crev = multiBuild.cargo-crev;

          ci = {
            cargo-crev = multiBuild.cargo-crev;
            workspace = multiBuild.workspace;
            workspaceDeps = multiBuild.workspaceDeps;
            clippy = multiBuild.clippy;
            tests = multiBuild.tests;
          };

          container = {
            cargo-crev = cargo-crev-container;
          };
        };

        legacyPackages = multiBuild;

        devShells = flakeboxLib.mkShells {
          packages = [ ];
          shellHook = ''
            # auto-install git hooks
            dot_git="$(git rev-parse --git-common-dir)"
            if [[ ! -d "$dot_git/hooks" ]]; then mkdir "$dot_git/hooks"; fi
            for hook in misc/git-hooks/* ; do ln -sf "$(pwd)/$hook" "$dot_git/hooks/" ; done
            ${pkgs.git}/bin/git config commit.template misc/git-hooks/commit-template.txt
          '';
        };
      }
    );
}
