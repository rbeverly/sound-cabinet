#!/usr/bin/env bash
# sound-cabinet installer. Detects platform, resolves a release tag,
# downloads the matching binary + .sha256, verifies, and installs to PATH.
set -euo pipefail
IFS=$'\n\t'

CURRENT_STEP="boot"
trap '_rc=$?; echo "ERROR at line $LINENO (step: ${CURRENT_STEP:-unknown})" >&2; if [[ -n "${TMPDIR_WORKSPACE:-}" && -d "${TMPDIR_WORKSPACE:-}" ]]; then echo "(workspace preserved at $TMPDIR_WORKSPACE for inspection)" >&2; fi; exit $_rc' ERR

readonly REPO_OWNER="rbeverly"
readonly REPO_NAME="sound-cabinet"
readonly BINARY_NAME="sound-cabinet"
readonly DEFAULT_API_BASE="https://api.github.com"
readonly DEFAULT_RELEASES_BASE="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download"

VERSION="" PRERELEASE=0 PREFIX="" USER_INSTALL=0 NON_INTERACTIVE=0 DRY_RUN=0
API_BASE="$DEFAULT_API_BASE"; RELEASES_BASE="$DEFAULT_RELEASES_BASE"
OS=""; TRIPLE=""; SHA256_VERIFY=(); TMPDIR_WORKSPACE=""
ALSA_HINT_NEEDED=0; ALSA_HINT_CMD=""
PATH_HINT_NEEDED=0; PATH_HINT_PATH=""

usage() {
  cat <<EOF
Usage: install.sh [options]
  --version vX.Y.Z         Pin a specific release tag
  --prerelease             Include pre-release tags in version selection
  --prefix <path>          Override install directory
  --user                   Install to \$HOME/.local/bin (no sudo)
  --non-interactive, -y    Skip prompts, use defaults
  --dry-run                Print actions without executing destructive ones
  --api-base-url <url>     Override GitHub API base (for testing)
  --releases-base-url <u>  Override release download base (for testing)
  --help                   Show this help and exit
EOF
}

parse_args() {
  CURRENT_STEP="parse_args"
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --version) VERSION="${2:-}"; shift 2 ;;
      --prerelease) PRERELEASE=1; shift ;;
      --prefix) PREFIX="${2:-}"; shift 2 ;;
      --user) USER_INSTALL=1; shift ;;
      --non-interactive|-y) NON_INTERACTIVE=1; shift ;;
      --dry-run) DRY_RUN=1; shift ;;
      --api-base-url) API_BASE="${2:-}"; shift 2 ;;
      --releases-base-url) RELEASES_BASE="${2:-}"; shift 2 ;;
      --help|-h) usage; exit 0 ;;
      *) echo "Unknown flag: $1" >&2; usage >&2; exit 2 ;;
    esac
  done
  if [[ -n "$VERSION" && $PRERELEASE -eq 1 ]]; then
    echo "ERROR: --version and --prerelease cannot be combined." >&2; exit 2
  fi
}

detect_target_triple() {
  CURRENT_STEP="detect_target_triple"
  OS=$(uname -s); local arch; arch=$(uname -m)
  case "$arch" in x86_64|amd64) arch="x86_64" ;; arm64|aarch64) arch="aarch64" ;; esac
  case "${OS}/${arch}" in
    Linux/x86_64) TRIPLE="x86_64-unknown-linux-gnu" ;;
    Linux/aarch64) TRIPLE="aarch64-unknown-linux-gnu" ;;
    Darwin/aarch64) TRIPLE="aarch64-apple-darwin" ;;
    Darwin/x86_64) echo "ERROR: Intel Mac is out of scope for this installer; build from source per README." >&2; exit 1 ;;
    *) echo "ERROR: no pre-built binary for ${OS}/${arch}; build from source per README." >&2; exit 1 ;;
  esac
}

select_sha_tool() {
  if command -v sha256sum >/dev/null 2>&1; then SHA256_VERIFY=(sha256sum)
  elif command -v shasum >/dev/null 2>&1; then SHA256_VERIFY=(shasum -a 256)
  else echo "ERROR: need sha256sum or shasum to verify downloads." >&2; exit 1; fi
}

fetch_releases_json() {
  CURRENT_STEP="fetch_releases_json"
  curl -fsSL --proto =https --tlsv1.2 "${API_BASE}/repos/${REPO_OWNER}/${REPO_NAME}/releases"
}

# Reads JSON from stdin, emits one tag per line in publish order.
# $1: include_pre (1 to include prereleases, 0 to filter to stable SemVer)
parse_tags_from_json() {
  local include_pre="$1" current_tag="" is_pre line
  while IFS= read -r line; do
    case "$line" in
      *'"tag_name":'*)
        current_tag=$(sed -E 's/.*"tag_name":[[:space:]]*"([^"]+)".*/\1/' <<<"$line") ;;
      *'"prerelease":'*)
        is_pre=0; [[ "$line" == *"true"* ]] && is_pre=1
        if [[ -n "$current_tag" ]]; then
          if [[ "$include_pre" == "1" ]]; then echo "$current_tag"
          elif [[ "$is_pre" == "0" && "$current_tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then echo "$current_tag"
          fi
        fi
        current_tag="" ;;
    esac
  done < <(grep -E '"tag_name":|"prerelease":')
}

select_version() {
  CURRENT_STEP="select_version"
  local json filtered all default top5 reply sel i t
  json=$(fetch_releases_json)
  filtered=$(echo "$json" | parse_tags_from_json "$PRERELEASE")
  all=$(echo "$json" | parse_tags_from_json 1)
  if [[ -n "$VERSION" ]]; then
    grep -qx -- "$VERSION" <<<"$all" || { echo "ERROR: tag '$VERSION' not found in published releases." >&2; exit 1; }
    echo "$VERSION"; return
  fi
  [[ -z "$filtered" ]] && { echo "ERROR: no matching release tags found." >&2; exit 1; }
  default=$(head -n1 <<<"$filtered")
  if [[ $NON_INTERACTIVE -eq 1 || ! -e /dev/tty ]]; then echo "$default"; return; fi
  top5=$(head -n5 <<<"$filtered")
  {
    echo ""; echo "Available ${BINARY_NAME} releases:"
    i=0
    while IFS= read -r t; do i=$((i+1)); echo "  [$i] $t"; done <<<"$top5"
    printf "Choose a version to install. Default [1] (%s): " "$default"
  } > /dev/tty
  reply=""; read -r reply < /dev/tty || reply=""
  [[ -z "$reply" ]] && { echo "$default"; return; }
  if [[ "$reply" =~ ^[0-9]+$ ]]; then
    sel=$(sed -n "${reply}p" <<<"$top5")
    [[ -z "$sel" ]] && { echo "ERROR: invalid selection: $reply" >&2; exit 1; }
    echo "$sel"; return
  fi
  grep -qx -- "$reply" <<<"$filtered" || { echo "ERROR: tag '$reply' is not in the available releases." >&2; exit 1; }
  echo "$reply"
}

determine_install_path() {
  CURRENT_STEP="determine_install_path"
  if [[ -n "$PREFIX" ]]; then echo "$PREFIX"; return; fi
  if [[ $USER_INSTALL -eq 1 ]]; then echo "${HOME}/.local/bin"; return; fi
  if command -v sudo >/dev/null 2>&1 || [[ "$(id -u)" == "0" ]]; then echo "/usr/local/bin"; return; fi
  echo "Notice: sudo unavailable; installing to \$HOME/.local/bin instead." >&2
  echo "${HOME}/.local/bin"
}

linux_alsa_check() {
  CURRENT_STEP="linux_alsa_check"
  [[ "$OS" != "Linux" ]] && return 0
  ldconfig -p 2>/dev/null | grep -q 'libasound\.so\.2' && return 0
  local _id="" _id_like=""
  if [[ -r /etc/os-release ]]; then
    _id=$(. /etc/os-release 2>/dev/null; echo "${ID:-}")
    _id_like=$(. /etc/os-release 2>/dev/null; echo "${ID_LIKE:-}")
  fi
  case " $_id $_id_like " in
    *debian*|*ubuntu*) ALSA_HINT_CMD="sudo apt-get install -y libasound2" ;;
    *fedora*|*rhel*|*centos*) ALSA_HINT_CMD="sudo dnf install -y alsa-lib" ;;
    *arch*|*manjaro*) ALSA_HINT_CMD="sudo pacman -S --noconfirm alsa-lib" ;;
    *) ALSA_HINT_CMD="install libasound2 (Debian/Ubuntu) or alsa-lib (Fedora/Arch) via your package manager" ;;
  esac
  ALSA_HINT_NEEDED=1
  echo "! ALSA runtime library not detected. Run: ${ALSA_HINT_CMD}"
}

dry_or_run() {
  if [[ $DRY_RUN -eq 1 ]]; then
    printf '[dry-run]'; printf ' %s' "$@"; printf '\n'
  else
    "$@"
  fi
}

download_and_verify() {
  CURRENT_STEP="download_and_verify"
  local tag="$1" tmpdir="$2"
  local base="${BINARY_NAME}-${tag}-${TRIPLE}"
  local binary_url="${RELEASES_BASE}/${tag}/${base}"
  local checksum_url="${binary_url}.sha256"
  cd "$tmpdir"
  dry_or_run curl -fSL --proto =https --tlsv1.2 -o "${base}" "${binary_url}"
  dry_or_run curl -fSL --proto =https --tlsv1.2 -o "${base}.sha256" "${checksum_url}"
  if [[ $DRY_RUN -eq 1 ]]; then echo "[dry-run] ${SHA256_VERIFY[*]} -c ${base}.sha256"; return; fi
  if ! "${SHA256_VERIFY[@]}" -c "${base}.sha256" >/dev/null 2>&1; then
    local computed expected
    computed=$("${SHA256_VERIFY[@]}" "$base" | awk '{print $1}')
    expected=$(awk '{print $1}' "${base}.sha256")
    echo "ERROR: SHA-256 verification failed." >&2
    echo "  computed: $computed" >&2
    echo "  expected: $expected" >&2
    echo "  workspace preserved at: $tmpdir" >&2
    exit 1
  fi
}

install_binary() {
  CURRENT_STEP="install_binary"
  local tag="$1" tmpdir="$2" install_path="$3"
  local src="${tmpdir}/${BINARY_NAME}-${tag}-${TRIPLE}"
  local dst="${install_path}/${BINARY_NAME}"
  local maybe_sudo=()
  if [[ "$install_path" == "/usr/local/bin" && "$(id -u)" != "0" ]]; then
    maybe_sudo=(sudo)
  fi
  dry_or_run "${maybe_sudo[@]}" mkdir -p "$install_path"
  dry_or_run "${maybe_sudo[@]}" install -m 755 "$src" "$dst"
}

path_check_and_hint() {
  CURRENT_STEP="path_check_and_hint"
  local install_path="$1"
  case "$install_path" in "$HOME"*) ;; *) return 0 ;; esac
  case ":$PATH:" in *":$install_path:"*) return 0 ;; esac
  PATH_HINT_NEEDED=1; PATH_HINT_PATH="$install_path"
  echo ""
  echo "Note: $install_path is not in your PATH. Append one of:"
  echo "  bash: echo 'export PATH=\"$install_path:\$PATH\"' >> ~/.bashrc"
  echo "  zsh:  echo 'export PATH=\"$install_path:\$PATH\"' >> ~/.zshrc"
  echo "  fish: fish_add_path $install_path"
}

final_summary() {
  CURRENT_STEP="final_summary"
  local tag="$1" install_path="$2"
  echo ""
  echo "Installed ${BINARY_NAME} ${tag} to ${install_path}/${BINARY_NAME}"
  echo "Try: ${BINARY_NAME} --help"
  if [[ $ALSA_HINT_NEEDED -eq 1 ]]; then
    echo "! ALSA runtime library not detected. Run: ${ALSA_HINT_CMD}"
  fi
  if [[ $PATH_HINT_NEEDED -eq 1 ]]; then
    echo "Reminder: add ${PATH_HINT_PATH} to your shell PATH (snippet above)."
  fi
}

main() {
  parse_args "$@"
  detect_target_triple
  select_sha_tool
  local tag install_path
  tag=$(select_version) || exit $?
  install_path=$(determine_install_path) || exit $?
  TMPDIR_WORKSPACE=$(mktemp -d)
  linux_alsa_check
  download_and_verify "$tag" "$TMPDIR_WORKSPACE"
  install_binary "$tag" "$TMPDIR_WORKSPACE" "$install_path"
  path_check_and_hint "$install_path"
  [[ $DRY_RUN -eq 0 ]] && { rm -rf "$TMPDIR_WORKSPACE"; TMPDIR_WORKSPACE=""; }
  final_summary "$tag" "$install_path"
}

main "$@"
