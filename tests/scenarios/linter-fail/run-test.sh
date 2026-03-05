#!/usr/bin/env bash
# Test that the let-fn linter correctly detects violations
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
LINTER="$REPO_ROOT/result/bin/nix-let-fn-linter"

if [[ ! -x "$LINTER" ]]; then
  echo "Error: Linter not found at $LINTER"
  echo "Run: nix build .#nix-let-fn-linter"
  exit 1
fi

FAILED=0

echo "=== Testing nix-let-fn-linter ==="
echo ""

# Test let-fn violations
echo "--- let-fn-violation.nix ---"
OUTPUT=$("$LINTER" "$SCRIPT_DIR/let-fn-violation.nix" 2>&1) || true
echo "$OUTPUT"
if echo "$OUTPUT" | grep -q "function definition in let"; then
  echo "✓ let-fn rule triggered correctly"
else
  echo "✗ let-fn rule NOT triggered"
  FAILED=1
fi
echo ""

# Summary
echo "=== Summary ==="
if [[ $FAILED -eq 0 ]]; then
  echo "All linter tests passed!"
  exit 0
else
  echo "Some linter tests failed!"
  exit 1
fi
