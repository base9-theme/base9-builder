#!/bin/bash
wasm-pack build --target web
wasm-opt -Oz pkg/base9_builder_bg.wasm
cargo build --release

scripts/monkey_patch_wasm.sh
