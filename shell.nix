let
  pkgs = import ./.;
in
pkgs.mkShell {
  name = "turbocheck";
  buildInputs = with pkgs; [
    rustToolchain
    cargo-bloat
    cargo-edit
    cargo-release
    cargo-udeps
    niv
  ];
}
