#!/bin/bash

set -e
set -x

if [[ -n $CHANNEL ]]; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain $CHANNEL -y
fi

rm -f db.json.gz db2.json.gz
rm -f db2.json

# Basic tests
# cargo +nightly fmt -- --write-mode=diff
cargo build $FEATURES
cargo test $FEATURES
cargo bench $FEATURES

# Test command-line interface
cargo run $FEATURES -- build db.json.gz README.md
cargo run $FEATURES -- check db.json.gz README.md
cargo run $FEATURES -- build db2.json.gz README.md
( ! cargo run $FEATURES -- build db.json.gz README.md ) && true || false # shouldn't overwrite
cargo run $FEATURES -- build db.json.gz README.md -f # should overwrite
cargo run $FEATURES -- diff db.json.gz db2.json.gz
cargo run $FEATURES -- selfcheck db.json.gz
gunzip db2.json.gz && echo "asdf" >> db2.json && gzip db2.json
( ! cargo run $FEATURES -- selfcheck db2.json.gz ) && true || false
