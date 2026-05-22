#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"
export MOCK_UNAME_S=Linux MOCK_UNAME_M=x86_64
setup_test
run_install --non-interactive --dry-run --prefix "$TEST_TMPDIR/bin"
assert_exit_code 0
assert_contains "$TEST_OUTPUT" "[dry-run]"
assert_file_missing "$TEST_TMPDIR/bin/sound-cabinet"
assert_file_missing "$HOME/.local/bin/sound-cabinet"
teardown_test
