#!/bin/bash

set -e
set -x

rm -f db.json.gz db2.json.gz
rm -f db2.json

# Basic tests
# cargo +nightly fmt -- --write-mode=diff
cargo build $FEATURES
cargo test $FEATURES
cargo bench $FEATURES

# Test command-line interface
cargo run $FEATURES -- build db README.md
cargo run $FEATURES -- check db README.md
cargo run $FEATURES -- build db2 README.md
( ! cargo run $FEATURES -- build db README.md ) && true || false # shouldn't overwrite
cargo run $FEATURES -- build db README.md -f # should overwrite
cargo run $FEATURES -- diff db db2
cargo run $FEATURES -- selfcheck db
gunzip db2.json.gz && echo "asdf" >> db2.json && gzip db2.json
( ! cargo run $FEATURES -- selfcheck db2 ) && true || false
