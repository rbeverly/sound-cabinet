#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"
export MOCK_UNAME_S=Linux MOCK_UNAME_M=x86_64
setup_test
run_install --prerelease --non-interactive --dry-run
assert_exit_code 0
# Most-recent published in the fixture is v0.6.0-rc1 (a prerelease)
assert_contains "$TEST_OUTPUT" "v0.6.0-rc1"
teardown_test
