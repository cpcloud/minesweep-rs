{
  description = "A minesweeper game, written in Rust";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";

    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable-small";

    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };

    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };

    naersk = {
      url = "github:nmattia/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    gitignore = {
      url = "github:hercules-ci/gitignore.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { self
    , fenix
    , flake-utils
    , gitignore
    , naersk
    , nixpkgs
    , pre-commit-hooks
    , ...
    }:
    flake-utils.lib.eachDefaultSystem (localSystem:
    let
      crossSystem = nixpkgs.lib.systems.examples.musl64 // { useLLVM = false; };
      pkgs = import nixpkgs {
        inherit localSystem crossSystem;
        overlays = [
          fenix.overlays.default
          gitignore.overlay
          naersk.overlay
          (final: prev: {
            rustToolchain =
              let
                fenixPackages = fenix.packages.${localSystem};
              in
              final.fenix.combine [
                fenixPackages.latest.clippy-preview
                fenixPackages.latest.rust-analysis
                fenixPackages.latest.rust-analyzer-preview
                fenixPackages.latest.rust-src
                fenixPackages.latest.rust-std
                fenixPackages.latest.rustfmt-preview
                fenixPackages.minimal.cargo
                fenixPackages.minimal.rustc
                final.fenix.targets.${crossSystem.config}.latest.rust-std
              ];

            rustStdenv = final.pkgsBuildHost.llvmPackages_14.stdenv;
            rustLinker = final.pkgsBuildHost.llvmPackages_14.lld;

            naerskBuild = (prev.pkgsBuildHost.naersk.override {
              cargo = final.rustToolchain;
              rustc = final.rustToolchain;
              stdenv = final.rustStdenv;
            }).buildPackage;

            prettierTOML = final.pkgsBuildHost.writeShellScriptBin "prettier" ''
              ${final.pkgsBuildHost.nodePackages.prettier}/bin/prettier \
              --plugin-search-dir "${final.pkgsBuildHost.nodePackages.prettier-plugin-toml}/lib" \
              "$@"
            '';
          })
        ];
      };
      inherit (pkgs.lib) mkForce;
    in
    rec {
      packages.minesweep = pkgs.naerskBuild {
        pname = "minesweep";
        src = pkgs.gitignoreSource ./.;

        nativeBuildInputs = with pkgs; [ rustStdenv.cc rustLinker ];

        CARGO_BUILD_TARGET = crossSystem.config;

        RUSTFLAGS = "-C linker-flavor=ld.lld -C target-feature=+crt-static";
      };

      packages.default = packages.minesweep;

      packages.minesweep-image = pkgs.pkgsBuildBuild.dockerTools.buildLayeredImage {
        name = "minesweep";
        config = {
          Entrypoint = [ "${packages.minesweep}/bin/minesweep" ];
          Command = [ "${packages.minesweep}/bin/minesweep" ];
        };
      };

      apps.minesweep = flake-utils.lib.mkApp { drv = packages.minesweep; };
      apps.default = apps.minesweep;

      checks = {
        pre-commit-check = pre-commit-hooks.lib.${localSystem}.run {
          src = ./.;
          hooks = {
            nix-linter.enable = true;
            nixpkgs-fmt.enable = true;
            shellcheck.enable = true;

            shfmt = {
              enable = true;
              entry = mkForce "${pkgs.pkgsBuildBuild.shfmt}/bin/shfmt -i 2 -sr -d -s -l";
              files = "\\.sh$";
            };

            rustfmt = {
              enable = true;
              entry = mkForce "${pkgs.pkgsBuildBuild.rustToolchain}/bin/cargo fmt -- --check --color=always";
            };

            clippy = {
              enable = true;
              entry = mkForce "${pkgs.pkgsBuildBuild.rustToolchain}/bin/cargo clippy -- -D warnings";
            };

            cargo-check = {
              enable = true;
              entry = mkForce "${pkgs.pkgsBuildBuild.rustToolchain}/bin/cargo check";
            };

            prettier = {
              enable = true;
              entry = mkForce "${pkgs.pkgsBuildBuild.prettierTOML}/bin/prettier --check";
              types_or = [ "json" "toml" "yaml" "markdown" ];
            };
          };
        };
      };

      devShells.default = pkgs.mkShell {
        inputsFrom = [ packages.minesweep ];
        nativeBuildInputs = with pkgs.pkgsBuildBuild; [
          cacert
          cargo-audit
          cargo-bloat
          cargo-edit
          cargo-udeps
          file
          git
          nix-linter
          nixpkgs-fmt
          prettierTOML
        ];

        shellHook = self.checks.${localSystem}.pre-commit-check.shellHook;
      };
    });
}
