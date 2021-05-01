let
  pkgs = import ./.;
in
pkgs.mkShell {
  name = "sweep";
  buildInputs = with pkgs; [
    cargo-bloat
    cargo-edit
    cargo-release
    cargo-udeps
    niv
    rustToolchain
  ];
}
