#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"
export MOCK_UNAME_S=Linux MOCK_UNAME_M=x86_64
setup_test
# Drop NON_INTERACTIVE but redirect /dev/tty to /dev/null-like to simulate no
# controlling tty. We run install.sh with stdin closed and stdout/stderr piped
# (already done by run_install); /dev/tty still exists at the syscall level but
# bash's `[ -e /dev/tty ]` may still be true. Force the no-tty branch by
# bind-mounting... we can't. Use setsid -w to detach from tty instead.
set +e
TEST_OUTPUT="$(setsid -w bash "$REPO_ROOT/install.sh" \
  --api-base-url "http://test.local" \
  --releases-base-url "http://test.local/releases/download" \
  --dry-run </dev/null 2>&1)"
TEST_EXIT_CODE=$?
set -e
assert_exit_code 0
assert_contains "$TEST_OUTPUT" "v0.5.0"
assert_not_contains "$TEST_OUTPUT" "Choose a version"
teardown_test
