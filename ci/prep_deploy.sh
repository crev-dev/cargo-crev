#!/bin/bash

set -x

export CRATE_NAME=cargo-crev
name="$TRAVIS_TAG-$TARGET"
mkdir -p "$name"
cp "target/$TARGET/release/$CRATE_NAME" "$name/"
cp cargo-crev/README.md LICENSE* "$name/"
tar czvf "$name.tar.gz" "$name"

# Get the sha-256 checksum w/o filename and newline
echo -n $(shasum -ba 256 "$name.tar.gz" | cut -d " " -f 1) > "$name.tar.gz.sha256"

