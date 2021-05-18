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
      inherit (self.fenix.latest)
        rustc
        cargo
        clippy-preview
        rustfmt-preview
        rust-analysis
        rust-analyzer-preview
        rust-std
        rust-src;

      rustToolchain = self.fenix.latest.withComponents [
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
      minesweep = self.naersk.buildPackage {
        root = ./.;
      };

      minesweepImage = self.dockerTools.buildLayeredImage {
        name = "minesweep";
        config = {
          Entrypoint = [ "${self.minesweep}/bin/minesweep" ];
        };
      };
    })
  ];
}
