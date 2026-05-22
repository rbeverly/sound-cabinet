#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"
export MOCK_UNAME_S=Linux MOCK_UNAME_M=aarch64
setup_test
run_install --non-interactive --dry-run
assert_exit_code 0
assert_contains "$TEST_OUTPUT" "aarch64-unknown-linux-gnu"
assert_contains "$TEST_OUTPUT" "v0.5.0"
assert_contains "$TEST_OUTPUT" "[dry-run]"
teardown_test
