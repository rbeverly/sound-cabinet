#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
export MOCK_UNAME_S=Darwin MOCK_UNAME_M=x86_64
setup_test
run_install --non-interactive --dry-run
assert_equals 0 "$TEST_EXIT_CODE"
assert_contains "$TEST_OUTPUT" "x86_64-apple-darwin"
assert_contains "$TEST_OUTPUT" "[dry-run]"
assert_contains "$TEST_OUTPUT" "v0.5.0"
assert_not_contains "$TEST_OUTPUT" "ALSA"
teardown_test
