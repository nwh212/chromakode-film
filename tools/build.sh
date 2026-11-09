#!/bin/sh
# Builds every filter into dist/: ChromaKode 64 (chromakode.wasm), ChromaGold 200 (chromagold.wasm),
# ChromaStill 800T (chromastill.wasm) and ChromaLome Purple (chromalome.wasm).
set -e
cd "$(dirname "$0")/.."
out=target/wasm32-unknown-unknown/release/chromakode.wasm
cargo build --release --target wasm32-unknown-unknown "$@"
cp "$out" dist/chromakode.wasm
cargo build --release --target wasm32-unknown-unknown --features gold "$@"
cp "$out" dist/chromagold.wasm
cargo build --release --target wasm32-unknown-unknown --features still "$@"
cp "$out" dist/chromastill.wasm
cargo build --release --target wasm32-unknown-unknown --features purple "$@"
cp "$out" dist/chromalome.wasm
ls -l dist/*.wasm
