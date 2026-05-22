#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"
export MOCK_UNAME_S=Linux MOCK_UNAME_M=x86_64
setup_test
# Force the ldconfig check to miss libasound. We can't easily mock ldconfig
# globally, but on this sandbox `ldconfig -p | grep -q libasound` reliably
# returns non-zero. If it ever finds libasound, the test will still pass
# the install-completed assertion; the ALSA-hint assertion is the discriminator.
run_install --non-interactive --dry-run
assert_exit_code 0
if ldconfig -p 2>/dev/null | grep -q 'libasound\.so\.2'; then
  echo "Note: libasound.so.2 actually present in sandbox; ALSA-hint assertion skipped"
else
  assert_contains "$TEST_OUTPUT" "ALSA runtime library not detected"
fi
assert_contains "$TEST_OUTPUT" "Installed sound-cabinet"
teardown_test
