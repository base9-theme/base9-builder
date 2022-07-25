#!/bin/bash

mkdir -p out

# release
cargo build --release
cp target/release/base9-builder out/base9-builder

# sample_mustache_data.min.json
cargo run list-variables - > out/mustache_data.min.json

# wasm
wasm-pack build --target web
wasm-opt -Oz pkg/base9_builder_bg.wasm
scripts/monkey_patch_wasm.sh
