let
  pkgs = import <nixpkgs> { };
  packages = with pkgs; [
    pkg-config
    openssl
    sqlx-cli
  ];
in
  pkgs.mkShell {
    buildInputs = packages;
    LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath packages}";
    RUST_LOG = "debug";
  }
