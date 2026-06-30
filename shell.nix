{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell {
  buildInputs = with pkgs; [
    cargo
    rustc
    rustfmt
    clippy
    rust-analyzer
    pkg-config
    gcc
  ];

  shellHook = ''
    echo "🦀 hare development shell"
    echo "  cargo  $(cargo --version | head -1)"
    echo "  rustc  $(rustc --version)"
  '';
}
