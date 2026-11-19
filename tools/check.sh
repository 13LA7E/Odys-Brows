#!/usr/bin/env bash
# One verify command for Linux / POSIX (PHASES.md P1-06).
# Runs formatting, linting, tests, JS checks, token checks, and visual checks.
#   bash tools/check.sh
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"

echo "=== 1/6: cargo fmt --check ==="
if ! cargo fmt --check; then
    echo "cargo fmt --check failed. Run 'cargo fmt' to fix." >&2
    exit 1
fi

echo "=== 2/6: cargo clippy ==="
if ! cargo clippy --all-targets -- -D warnings; then
    echo "cargo clippy failed with warnings." >&2
    exit 1
fi

echo "=== 3/6: cargo test ==="
if ! cargo test; then
    echo "cargo test failed." >&2
    exit 1
fi

echo "=== 4/6: JS syntax check (node --check) ==="
js_count=0
for f in "$root"/ui/*.js; do
    if [[ -f "$f" ]]; then
        if ! node --check "$f"; then
            echo "JS syntax check failed for $f" >&2
            exit 1
        fi
        js_count=$((js_count + 1))
    fi
done
echo "Checked $js_count JS files."

echo "=== 5/6: Token check (no raw hex colours outside tokens.css) ==="
bad_hex=0
for f in "$root"/ui/*.css; do
    if [[ -f "$f" && "$(basename "$f")" != "tokens.css" ]]; then
        if grep -En '#[0-9a-fA-F]{3,8}' "$f"; then
            bad_hex=1
        fi
    fi
done
if [[ $bad_hex -ne 0 ]]; then
    echo "Raw hex colours found outside ui/tokens.css" >&2
    exit 1
fi
echo "Token check passed: no raw hex colours found."

echo "=== 6/6: Visual check (fixture mode) ==="
py=""
if command -v python3 &>/dev/null && python3 -c "import playwright" 2>/dev/null; then
    py="python3"
elif command -v python &>/dev/null && python -c "import playwright" 2>/dev/null; then
    py="python"
fi

if [[ -n "$py" ]]; then
    if ! "$py" -u "$root/tools/visual_check.py" --report-only; then
        echo "Visual check failed." >&2
        exit 1
    fi
else
    echo "Playwright not installed; skipping visual check."
fi

echo ""
echo "All checks PASSED."
exit 0
