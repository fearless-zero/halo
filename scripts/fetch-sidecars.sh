#!/usr/bin/env bash
#
# Builds the two inference engines Halo bundles as Tauri sidecars:
#   - llama-server  (llama.cpp)  — runs the Qwen3 note writer
#   - whisper-cli   (whisper.cpp) — transcribes recordings
#
# They are compiled from source with CMake (enabling Metal on macOS, and CPU
# elsewhere) and copied into src-tauri/binaries/ with the Rust target-triple
# suffix Tauri expects for external binaries. Run this once before `npm run
# build` (or `npm run dev`). Requires: git, cmake, a C/C++ compiler.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN_DIR="$ROOT/src-tauri/binaries"
mkdir -p "$BIN_DIR"

if ! command -v rustc >/dev/null 2>&1; then
  echo "error: rustc not found (needed to determine the target triple)" >&2
  exit 1
fi
TRIPLE="$(rustc -Vv | awk '/host:/ {print $2}')"
EXT=""
case "$TRIPLE" in
  *windows*) EXT=".exe" ;;
esac

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

build_target() {
  local repo="$1" name="$2" target="$3"
  echo "==> Building $target from $repo"
  git clone --depth 1 "$repo" "$WORK/$name"
  cmake -S "$WORK/$name" -B "$WORK/$name/build" -DCMAKE_BUILD_TYPE=Release -DLLAMA_CURL=OFF >/dev/null
  cmake --build "$WORK/$name/build" --config Release --target "$target" -j
  local built
  built="$(find "$WORK/$name/build" -name "$target$EXT" -type f | head -1)"
  if [ -z "$built" ]; then
    echo "error: could not locate built $target" >&2
    exit 1
  fi
  cp "$built" "$BIN_DIR/$target-$TRIPLE$EXT"
  chmod +x "$BIN_DIR/$target-$TRIPLE$EXT" 2>/dev/null || true
  echo "    -> $BIN_DIR/$target-$TRIPLE$EXT"
}

build_target "https://github.com/ggml-org/llama.cpp"   "llama"   "llama-server"
build_target "https://github.com/ggml-org/whisper.cpp" "whisper" "whisper-cli"

echo "Sidecars ready in $BIN_DIR"
