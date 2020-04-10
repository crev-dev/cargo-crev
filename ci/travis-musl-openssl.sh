#!/bin/bash
set -ex

case "$TARGET" in
    x86_64-*)
        OPTIONS=(linux-x86_64)
        ;;
    i686-*)
        OPTIONS=(linux-generic32 -m32 -Wl,-melf_i386)
        ;;
esac

rustup target add "$TARGET"
curl https://www.openssl.org/source/openssl-1.1.1f.tar.gz | tar xzf -
cd openssl-1.1.1f
CC=musl-gcc ./Configure "-idirafter /usr/include/" "-idirafter /usr/include/x86_64-linux-gnu/" --prefix="$OPENSSL_DIR" no-dso no-ssl2 no-ssl3 "${OPTIONS[@]}" -fPIC
make -j"$(nproc)"
make install
