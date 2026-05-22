#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"
setup_test
run_install --help
assert_exit_code 0
assert_contains "$TEST_OUTPUT" "Usage: install.sh"
assert_contains "$TEST_OUTPUT" "--version"
assert_contains "$TEST_OUTPUT" "--prerelease"
assert_contains "$TEST_OUTPUT" "--user"
assert_contains "$TEST_OUTPUT" "--dry-run"
teardown_test
