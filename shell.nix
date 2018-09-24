# Run `nix-shell` to be able
# to build Grin on NixOS.
{ pkgs ? import <nixpkgs> {} }:

pkgs.stdenv.mkDerivation {
  name = "rdedup";

  buildInputs = with pkgs; [
    ncurses cmake gcc openssl libsodium lzma libsodium clang_39 zlib
  ];

  shellHook = ''
      LD_LIBRARY_PATH=${pkgs.ncurses}/lib/:$LD_LIBRARY_PATH
      LD_LIBRARY_PATH=${pkgs.openssl}/lib/:$LD_LIBRARY_PATH
      LD_LIBRARY_PATH=${pkgs.libsodium}/lib/:$LD_LIBRARY_PATH
      LD_LIBRARY_PATH=${pkgs.lzma}/lib/:$LD_LIBRARY_PATH
      LD_LIBRARY_PATH=${pkgs.zlib.dev}/lib/:$LD_LIBRARY_PATH
      LIBRARY_PATH=${pkgs.zlib}/lib/:$LIBRARY_PATH
      PKG_CONFIG_PATH=${pkgs.libsodium.dev}/lib/pkgconfig:$PKG_CONFIG_PATH
      PKG_CONFIG_PATH=${pkgs.lzma.dev}/lib/pkgconfig:$PKG_CONFIG_PATH
      LD_LIBRARY_PATH=${pkgs.llvmPackages.libclang}/lib/:$LD_LIBRARY_PATH
  '';
}

