#!/usr/bin/env bash
#
# Builds the two inference engines Halo bundles as Tauri sidecars:
#   - llama-server  (llama.cpp)  — runs the Qwen3 note writer
#   - whisper-cli   (whisper.cpp) — transcribes recordings
#
# They are compiled from source with CMake and copied into src-tauri/binaries/
# with the Rust target-triple suffix Tauri expects for external binaries.
#
# By default it builds for the host target triple (for `npm run dev` / a local
# `npm run build`). To build the arch-specific binaries a universal macOS
# release needs, set SIDECAR_TRIPLES:
#
#   SIDECAR_TRIPLES="aarch64-apple-darwin x86_64-apple-darwin" bash scripts/fetch-sidecars.sh
#
# Requires: git, cmake, a C/C++ compiler. Cross-arch macOS builds also need the
# matching Xcode command-line tools (present on GitHub macOS runners).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN_DIR="$ROOT/src-tauri/binaries"
mkdir -p "$BIN_DIR"

if ! command -v rustc >/dev/null 2>&1; then
  echo "error: rustc not found (needed to determine the target triple)" >&2
  exit 1
fi

HOST_TRIPLE="$(rustc -Vv | awk '/host:/ {print $2}')"
TRIPLES="${SIDECAR_TRIPLES:-$HOST_TRIPLE}"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# Emit the extra CMake flags for a given target triple. macOS slices are built
# with GGML_NATIVE off so the binary is portable across CPUs of that arch; the
# Intel slice also disables Metal (kept on for Apple Silicon).
cmake_flags_for() {
  case "$1" in
    aarch64-apple-darwin) echo "-DCMAKE_OSX_ARCHITECTURES=arm64 -DGGML_NATIVE=OFF" ;;
    x86_64-apple-darwin)  echo "-DCMAKE_OSX_ARCHITECTURES=x86_64 -DGGML_NATIVE=OFF -DGGML_METAL=OFF" ;;
    *) echo "" ;;
  esac
}

build_engine() {
  local repo="$1" name="$2" target="$3"
  echo "==> Cloning $repo"
  git clone --depth 1 "$repo" "$WORK/$name"
  for triple in $TRIPLES; do
    local ext=""
    case "$triple" in *windows*) ext=".exe" ;; esac
    local extra
    extra="$(cmake_flags_for "$triple")"
    local bdir="$WORK/$name/build-$triple"
    echo "==> Building $target for $triple $extra"
    # shellcheck disable=SC2086
    cmake -S "$WORK/$name" -B "$bdir" -DCMAKE_BUILD_TYPE=Release -DLLAMA_CURL=OFF $extra >/dev/null
    cmake --build "$bdir" --config Release --target "$target" -j
    local built
    built="$(find "$bdir" -name "$target$ext" -type f | head -1)"
    if [ -z "$built" ]; then
      echo "error: could not locate built $target for $triple" >&2
      exit 1
    fi
    cp "$built" "$BIN_DIR/$target-$triple$ext"
    chmod +x "$BIN_DIR/$target-$triple$ext" 2>/dev/null || true
    echo "    -> $BIN_DIR/$target-$triple$ext"
  done
}

build_engine "https://github.com/ggml-org/llama.cpp"   "llama"   "llama-server"
build_engine "https://github.com/ggml-org/whisper.cpp" "whisper" "whisper-cli"

echo "Sidecars ready in $BIN_DIR"
