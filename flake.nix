{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        packages = with pkgs; [
          (rust-bin.stable.latest.default.override {
            extensions = [
              "rust-src"
              "rust-analyzer"
            ];
          })
          pkg-config
          openssl
          sqlx-cli
          cargo-audit
          cargo-watch
          python3
          python3Packages.yt-dlp
          python3Packages.pyyaml
          python3Packages.curl-cffi # Needed for full yt-dlp support
          deno # Needed for full yt-dlp support
          pre-commit
          hadolint
        ];
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = packages;
          LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath packages}";
          RUST_LOG = "debug";
        };
      }
    );
}
