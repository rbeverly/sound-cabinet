#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"
export MOCK_UNAME_S=Linux MOCK_UNAME_M=x86_64
setup_test
run_install --version v0.4.0 --non-interactive --dry-run
assert_exit_code 0
assert_contains "$TEST_OUTPUT" "v0.4.0"
assert_contains "$TEST_OUTPUT" "sound-cabinet-v0.4.0-x86_64-unknown-linux-gnu.tar.gz"
assert_not_contains "$TEST_OUTPUT" "v0.5.0"
teardown_test
