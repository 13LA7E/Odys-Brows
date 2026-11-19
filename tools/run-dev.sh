#!/usr/bin/env bash
# Run the development build of the browser with CEF's runtime on LD_LIBRARY_PATH.
#   bash tools/run-dev.sh [--release] [extra args]
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
profile=debug
if [[ "${1:-}" == "--release" ]]; then profile=release; shift; fi
lib="$(find "$root/target/$profile/build" -path '*cef_linux*' -name libcef.so -print -quit 2>/dev/null || true)"
if [[ -z "$lib" ]]; then echo "CEF runtime not found. Run: cargo build" >&2; exit 1; fi
export LD_LIBRARY_PATH="$(dirname "$lib"):${LD_LIBRARY_PATH:-}"
exec "$root/target/$profile/constellations" "$@"
