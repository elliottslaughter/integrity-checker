#!/bin/bash

set -e
set -x

# cargo +nightly fmt -- --write-mode=diff
cargo build $FEATURES
cargo test $FEATURES
cargo bench $FEATURES
cargo run $FEATURES -- build db README.md
cargo run $FEATURES -- check db README.md
cargo run $FEATURES -- build db2 README.md
! cargo run $FEATURES -- build db README.md # shouldn't overwrite
cargo run $FEATURES -- build db README.md -f # should overwrite
cargo run $FEATURES -- diff db db2
cargo run $FEATURES -- selfcheck db
