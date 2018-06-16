#!/bin/bash

set -e
set -x

rm -f db.json.gz db2.json.gz min_db.json.gz max_db.json.gz
rm -f db2.json

# # Basic tests
# # cargo +nightly fmt -- --write-mode=diff
# cargo build $FEATURES
# cargo test $FEATURES
# cargo bench $FEATURES

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

# Test files with different features are readable
MIN_FEATURES="--no-default-features"
MAX_FEATURES="--no-default-features --features=sha2-512256,blake2b,asm"
cargo run $MIN_FEATURES -- build min_db README.md
cargo run $MAX_FEATURES -- build max_db README.md
cargo run $FEATURES -- diff min_db db
cargo run $FEATURES -- diff max_db db
