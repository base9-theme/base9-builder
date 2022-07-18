#!/bin/bash
wasm-pack build --target web
cargo build --release

scripts/monkey_patch_wasm.sh
