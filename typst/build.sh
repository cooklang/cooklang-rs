#!/usr/bin/env sh
# Build the WebAssembly plugin used by the Typst package.
set -eu
cd "$(dirname "$0")/cooklang-core"
cargo build --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/cooklang_core.wasm ../cooklang.wasm
echo "Wrote typst/cooklang.wasm"
