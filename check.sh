#!/bin/sh

set -ex

# Hard deny on all warnings. Covers rustc, clippy and rustdoc.
export CARGO_BUILD_WARNINGS=deny

cargo check
cargo check --target wasm32-unknown-unknown

cargo clippy
cargo clippy --target wasm32-unknown-unknown

# Unit tests
cargo test

# Spin up a relay and run `example-simple-demo` as an integration test
RELAY=127.0.0.1:3000 # NOTE: hard-coded in example-simple-demo
cargo build -p onlyfriends-relay -p example-simple-demo
cargo run -p onlyfriends-relay -- --bind $RELAY &
sleep 1 # Give relay time to start
cargo run -p example-simple-demo
kill -9 %1 # Kill relay

cargo fmt --check

set +x

echo
echo "╔═══════════════════════════════════════════╗"
echo "║             ALL CHECKS PASSED             ║"
echo "╚═══════════════════════════════════════════╝"
