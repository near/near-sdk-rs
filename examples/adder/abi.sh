#!/bin/bash
TARGET="${CARGO_TARGET_DIR:-../../target}"
set -e
cd "$(dirname $0)"

# Build the lib
CARGO_PROFILE_DEV_OPT_LEVEL=0 \
    CARGO_PROFILE_DEV_DEBUG=0 \
    CARGO_PROFILE_DEV_LTO=off \
    RUSTFLAGS="-Awarnings" \
    cargo build --features near-sdk/__abi-generate --release

OS="$(uname)"
if [[ "$OS" == "Linux" ]]; then
    EXT=".so"
elif [[ "$OS" == "Darwin" ]]; then
    EXT=".dylib"
elif [[ "$OS" == MINGW* ]] || [[ "$OS" == CYGWIN* ]] || [[ "$OS" == MSYS* ]]; then
    EXT=".dll"
else
    echo "Unsupported OS: $OS"
    exit 1
fi

cp "$TARGET/release/libadder$EXT" ./res/
