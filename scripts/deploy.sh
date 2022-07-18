#!/bin/bash
cargo build --release

wasm-pack build --target web
scripts/monkey_patch_wasm.sh
