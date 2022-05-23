#!/bin/bash
set -e

RUSTFLAGS='-C link-arg=-s' cargo +stable build --all --target wasm32-unknown-unknown --release
cp -p target/wasm32-unknown-unknown/release/nearX.wasm res/
