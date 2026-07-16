#!/usr/bin/env bash
#
# Runs the full coverage suite and fails if either side is below 100%.
#   - Frontend: Vitest v8 coverage (thresholds enforced in vitest.config.ts).
#   - Backend:  cargo +nightly llvm-cov; fails if any source line is uncovered.
#     I/O boundaries (GUI loop, hardware audio, subprocess, OS clipboard) are
#     excluded at the function level via #[cfg_attr(coverage_nightly, coverage(off))].
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "== Frontend coverage (100% enforced) =="
npm run coverage

echo "== Backend coverage =="
TRIPLE="$(rustc -Vv | awk '/host:/ {print $2}')"
EXT=""
[[ "$TRIPLE" == *windows* ]] && EXT=".exe"
mkdir -p src-tauri/binaries
: > "src-tauri/binaries/llama-server-$TRIPLE$EXT"
: > "src-tauri/binaries/whisper-cli-$TRIPLE$EXT"
trap 'rm -f "src-tauri/binaries/llama-server-$TRIPLE$EXT" "src-tauri/binaries/whisper-cli-$TRIPLE$EXT"' EXIT

cargo +nightly llvm-cov --manifest-path src-tauri/Cargo.toml --lib --lcov --output-path "$ROOT/lcov.info"

UNCOVERED="$(awk -F'[:,]' '/^SF:/{f=$2} /^DA:/{if($3==0) print f":"$2}' "$ROOT/lcov.info")"
if [ -n "$UNCOVERED" ]; then
  echo "Uncovered backend lines:"
  echo "$UNCOVERED"
  exit 1
fi
echo "Backend: 100% line coverage."
