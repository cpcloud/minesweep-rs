{
  description = "A minesweeper game, written in Rust";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";

    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable-small";

    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };

    naersk = {
      url = "github:nmattia/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
  };

  outputs =
    { self
    , nixpkgs
    , flake-utils
    , pre-commit-hooks
    , naersk
    , fenix
    }:
    flake-utils.lib.eachDefaultSystem (system:
    let
      target = "x86_64-unknown-linux-musl";
      pkgs = nixpkgs.legacyPackages.${system};
      rustToolchain = with fenix.packages.${system}; combine [
        latest.clippy-preview
        latest.rust-analysis
        latest.rust-analyzer-preview
        latest.rust-src
        latest.rust-std
        latest.rustfmt-preview
        minimal.cargo
        minimal.rustc
        targets.${target}.latest.rust-std
      ];
      naersk-lib = naersk.lib.${system}.override {
        cargo = rustToolchain;
        rustc = rustToolchain;
      };
      inherit (pkgs.lib) mkForce;
    in
    rec {
      packages.minesweep = naersk-lib.buildPackage {
        pname = "minesweep";
        src = ./.;

        nativeBuildInputs = with pkgs.llvmPackages_12; [ clang lld ];

        dontPatchELF = true;

        CARGO_BUILD_TARGET = target;
        CARGO_BUILD_RUSTFLAGS = "-C linker-flavor=ld.lld -C target-feature=+crt-static";

        doCheck = true;
      };

      defaultPackage = packages.minesweep;

      apps.minesweep = flake-utils.lib.mkApp {
        drv = packages.minesweep;
      };
      defaultApp = apps.minesweep;

      packages.minesweep-image = pkgs.dockerTools.buildLayeredImage {
        name = "minesweep";
        config = {
          Entrypoint = [ "${packages.minesweep}/bin/minesweep" ];
        };
      };

      checks = {
        pre-commit-check = pre-commit-hooks.lib.${system}.run {
          src = ./.;
          hooks = {
            nix-linter = {
              enable = true;
              entry = mkForce "${pkgs.nix-linter}/bin/nix-linter";
              excludes = [ "nix/sources.nix" ];
            };

            nixpkgs-fmt = {
              enable = true;
              entry = mkForce "${pkgs.nixpkgs-fmt}/bin/nixpkgs-fmt --check";
            };

            shellcheck = {
              enable = true;
              entry = mkForce "${pkgs.shellcheck}/bin/shellcheck";
              files = "\\.sh$";
              types_or = mkForce [ ];
            };

            shfmt = {
              enable = true;
              entry = mkForce "${pkgs.shfmt}/bin/shfmt -i 2 -sr -d -s -l";
              files = "\\.sh$";
            };

            rustfmt = {
              enable = true;
              entry = mkForce "${rustToolchain}/bin/cargo fmt -- --check --color=always";
            };

            clippy = {
              enable = true;
              entry = mkForce "${rustToolchain}/bin/cargo clippy";
            };

            cargo-check = {
              enable = true;
              entry = mkForce "${rustToolchain}/bin/cargo check";
            };
          };

        };
      };

      devShell = pkgs.mkShell {
        nativeBuildInputs = (with pkgs; [
          cacert
          cargo-bloat
          cargo-edit
          cargo-udeps
          jq
          util-linux
          yj
          commitizen
        ]) ++ [
          rustToolchain
        ];
        shellHook = ''
          ${self.checks.${system}.pre-commit-check.shellHook}
        '';
      };
    });
}
