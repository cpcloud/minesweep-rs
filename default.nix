let
  sources = import ./nix/sources.nix;
in
import sources.nixpkgs {
  overlays = [
    (import sources.fenix)
    (self: super: {
      naersk = self.callPackage sources.naersk { };
    })
    (self: super: {
      inherit (self.rust-nightly.latest)
        rustc
        cargo
        clippy-preview
        rustfmt-preview
        rust-analysis
        rust-analyzer-preview
        rust-std
        rust-src;

      rustToolchain = self.rust-nightly.latest.withComponents [
        "rustc"
        "cargo"
        "clippy-preview"
        "rustfmt-preview"
        "rust-analysis"
        "rust-analyzer-preview"
        "rust-std"
        "rust-src"
      ];
    })
    (self: super: {
      sweep = self.naersk.buildPackage {
        root = ./.;
      };

      sweepImage = self.dockerTools.buildLayeredImage {
        name = "sweep";
        config = {
          Entrypoint = [ "${self.sweep}/bin/sweep" ];
        };
      };
    })
  ];
}
