#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"
export MOCK_UNAME_S=Linux MOCK_UNAME_M=armv7l
setup_test
run_install --non-interactive --dry-run
[[ $TEST_EXIT_CODE -ne 0 ]] || _fail "expected non-zero exit on unknown arch"
assert_contains "$TEST_OUTPUT" "no pre-built binary"
assert_contains "$TEST_OUTPUT" "build from source"
teardown_test
