# Shared helpers for install.sh tests. Source this from each *.test.sh file.

REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
FIXTURES_DIR="$REPO_ROOT/tests/install/fixtures"

TEST_OUTPUT=""
TEST_EXIT_CODE=0
TEST_TMPDIR=""
TEST_FAILED=0

setup_test() {
  TEST_TMPDIR="$(mktemp -d)"
  TMPDIR="$TEST_TMPDIR"
  export TMPDIR
  export HOME="$TEST_TMPDIR/home"
  mkdir -p "$HOME"
  export SUDO_LOG="$TEST_TMPDIR/sudo.log"
  : > "$SUDO_LOG"
  export PATH="$FIXTURES_DIR:$PATH"
  export MOCK_UNAME_S="${MOCK_UNAME_S:-Linux}"
  export MOCK_UNAME_M="${MOCK_UNAME_M:-x86_64}"
  unset CURL_404_BINARY CURL_404_CHECKSUM CURL_BAD_CHECKSUM
}

teardown_test() {
  if [[ $TEST_FAILED -eq 0 && -n "$TEST_TMPDIR" && -d "$TEST_TMPDIR" ]]; then
    rm -rf "$TEST_TMPDIR"
  fi
}

run_install() {
  set +e
  TEST_OUTPUT="$(bash "$REPO_ROOT/install.sh" \
    --api-base-url "http://test.local" \
    --releases-base-url "http://test.local/releases/download" \
    "$@" 2>&1)"
  TEST_EXIT_CODE=$?
  set -e
}

_fail() {
  TEST_FAILED=1
  echo "ASSERTION FAILED: $*" >&2
  if [[ -n "$TEST_OUTPUT" ]]; then
    echo "----- last output -----" >&2
    echo "$TEST_OUTPUT" >&2
    echo "-----------------------" >&2
  fi
  echo "(workspace preserved at $TEST_TMPDIR)" >&2
  exit 1
}

assert_contains() {
  local haystack="$1" needle="$2"
  case "$haystack" in *"$needle"*) return 0 ;; esac
  _fail "expected to find: $needle"
}

assert_not_contains() {
  local haystack="$1" needle="$2"
  case "$haystack" in *"$needle"*) _fail "expected NOT to find: $needle" ;; esac
}

assert_equals() {
  [[ "$1" == "$2" ]] || _fail "expected '$2', got '$1'"
}

assert_exit_code() {
  [[ "$TEST_EXIT_CODE" == "$1" ]] || _fail "expected exit $1, got $TEST_EXIT_CODE"
}

assert_file_exists() {
  [[ -e "$1" ]] || _fail "expected file to exist: $1"
}

assert_file_missing() {
  [[ -e "$1" ]] && _fail "expected file to be absent: $1"
  return 0
}
