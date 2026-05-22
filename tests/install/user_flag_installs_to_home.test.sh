#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"
export MOCK_UNAME_S=Linux MOCK_UNAME_M=x86_64
setup_test
run_install --user --non-interactive --dry-run
assert_exit_code 0
assert_contains "$TEST_OUTPUT" ".local/bin"
assert_not_contains "$TEST_OUTPUT" "[dry-run] sudo"
assert_not_contains "$TEST_OUTPUT" "/usr/local/bin/sound-cabinet"
teardown_test
