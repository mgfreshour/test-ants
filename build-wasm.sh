#!/usr/bin/env bash
set -euo pipefail

WASM_BINDGEN_VERSION="0.2.117"
TARGET="wasm32-unknown-unknown"
PROFILE="wasm-release"
OUT_DIR="web/out"

echo "==> Adding rustup target ${TARGET}"
rustup target add "${TARGET}"

echo "==> Building for ${TARGET} (profile: ${PROFILE})"
cargo build --profile "${PROFILE}" --target "${TARGET}"

# Install wasm-bindgen-cli if missing or wrong version
if ! command -v wasm-bindgen &>/dev/null || \
   [[ "$(wasm-bindgen --version 2>/dev/null)" != *"${WASM_BINDGEN_VERSION}"* ]]; then
    echo "==> Installing wasm-bindgen-cli ${WASM_BINDGEN_VERSION}"
    cargo install wasm-bindgen-cli --version "${WASM_BINDGEN_VERSION}"
fi

echo "==> Running wasm-bindgen"
mkdir -p "${OUT_DIR}"
wasm-bindgen \
    --out-dir "${OUT_DIR}" \
    --target web \
    "target/${TARGET}/${PROFILE}/colony.wasm"

echo "==> Copying assets to web/"
rsync -a --delete assets/ web/assets/

echo ""
echo "Build complete! Run ./serve-wasm.sh to test."
