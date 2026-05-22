#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"
export MOCK_UNAME_S=Linux MOCK_UNAME_M=x86_64
setup_test
export CURL_BAD_CHECKSUM=1
run_install --non-interactive --prefix "$TEST_TMPDIR/bin"
[[ $TEST_EXIT_CODE -ne 0 ]] || _fail "expected non-zero exit on sha mismatch"
assert_contains "$TEST_OUTPUT" "SHA-256 verification failed"
assert_contains "$TEST_OUTPUT" "workspace preserved at"
assert_file_missing "$TEST_TMPDIR/bin/sound-cabinet"
teardown_test
