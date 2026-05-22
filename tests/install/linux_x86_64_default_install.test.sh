#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"
export MOCK_UNAME_S=Linux MOCK_UNAME_M=x86_64
setup_test
run_install --non-interactive --dry-run
assert_exit_code 0
assert_contains "$TEST_OUTPUT" "x86_64-unknown-linux-gnu"
assert_contains "$TEST_OUTPUT" "v0.5.0"
assert_contains "$TEST_OUTPUT" "[dry-run]"
assert_contains "$TEST_OUTPUT" "/usr/local/bin"
assert_contains "$TEST_OUTPUT" "sudo install"
teardown_test
