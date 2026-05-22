#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"
export MOCK_UNAME_S=Darwin MOCK_UNAME_M=arm64
setup_test
run_install --non-interactive --dry-run
assert_exit_code 0
assert_contains "$TEST_OUTPUT" "aarch64-apple-darwin"
assert_contains "$TEST_OUTPUT" "v0.5.0"
assert_not_contains "$TEST_OUTPUT" "ALSA"
teardown_test
