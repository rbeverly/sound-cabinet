#!/usr/bin/env bash
# Test runner for tests/install/*.test.sh — executes each file in a subshell,
# reports PASS/FAIL per test, exits 1 if any failed.
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$REPO_ROOT"

if command -v shellcheck >/dev/null 2>&1; then
  echo "Running shellcheck..."
  shellcheck install.sh tests/install/*.sh tests/install/*.test.sh tests/install/fixtures/curl tests/install/fixtures/uname tests/install/fixtures/sudo || true
else
  echo "WARN: shellcheck not available in sandbox — skipping lint"
fi

passes=0
fails=0
failed_tests=()

for t in tests/install/*.test.sh; do
  [[ -e "$t" ]] || continue
  if bash "$t" >/dev/null 2>&1; then
    echo "PASS: $t"
    passes=$((passes + 1))
  else
    echo "FAIL: $t"
    bash "$t" || true
    fails=$((fails + 1))
    failed_tests+=("$t")
  fi
done

echo
echo "Results: $passes passed, $fails failed"
if [[ $fails -gt 0 ]]; then
  echo "Failed: ${failed_tests[*]}"
  exit 1
fi
