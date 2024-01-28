{ pkgs, lib }:
let
  toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
in
pkgs.mkShell rec {
  nativeBuildInputs = with pkgs; [
    (toolchain.override {
      extensions = ["rust-src" "clippy"];
    })
    rust-analyzer
    openssl
    pkg-config
    fontconfig
    protobuf
    clang
    rustfmt
    gnustep.libobjc
    alsa-lib
    wayland
    libxkbcommon
    libGL
  ];
  RUST_BACKTRACE = "1";
  LD_LIBRARY_PATH = "${lib.makeLibraryPath nativeBuildInputs}";
}
