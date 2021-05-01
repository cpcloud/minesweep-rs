let
  pkgs = import ./.;
in
pkgs.mkShell {
  name = "sweep";
  buildInputs = with pkgs; [
    cacert
    cargo-bloat
    cargo-edit
    cargo-release
    cargo-udeps
    git
    gitAndTools.gh
    jq
    niv
    rustToolchain
    util-linux
    yj
  ];
}
