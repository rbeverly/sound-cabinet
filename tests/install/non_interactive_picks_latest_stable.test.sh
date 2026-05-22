#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"
export MOCK_UNAME_S=Linux MOCK_UNAME_M=x86_64
setup_test
run_install --non-interactive --dry-run
assert_exit_code 0
# Latest stable per fixture is v0.5.0 (not v0.6.0-rc1, which is prerelease).
assert_contains "$TEST_OUTPUT" "v0.5.0"
assert_not_contains "$TEST_OUTPUT" "v0.6.0-rc1"
assert_not_contains "$TEST_OUTPUT" "Choose a version"
teardown_test
