#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"
# Simulate Darwin so the ALSA check is skipped entirely (the cleanest way to
# observe "no advisory printed" without depending on the host's ldconfig).
export MOCK_UNAME_S=Darwin MOCK_UNAME_M=arm64
setup_test
run_install --non-interactive --dry-run
assert_exit_code 0
assert_not_contains "$TEST_OUTPUT" "ALSA"
assert_not_contains "$TEST_OUTPUT" "libasound"
teardown_test
