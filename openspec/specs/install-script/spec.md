# install-script Specification

## Purpose
TBD - created by archiving change a02-install-script-and-wizard. Update Purpose after archive.
## Requirements
### Requirement: Curl-runnable bash installer at repo root

The repository SHALL include a file `install.sh` at the repository root that is invocable via the one-liner `curl -fsSL <raw-url>/install.sh | bash` and that, on successful execution, places the `sound-cabinet` binary at a writable directory on `PATH` (or returns a clear non-zero exit with a remediation message). The script SHALL use `#!/usr/bin/env bash`, SHALL enable strict mode (`set -euo pipefail`), and SHALL install an `ERR` trap that reports the current step name and the line number on any abort. The script SHALL NOT depend on any non-default tools beyond what ships on a fresh macOS install or a minimal Debian/Ubuntu/Fedora/Arch install: `curl`, `tar`, `uname`, `mktemp`, `install`, `grep`, `sed`, `id`, `sudo` (when available), `ldconfig` (Linux only), and one of `sha256sum` (Linux) or `shasum` (macOS).

#### Scenario: Default invocation on Linux x86_64 with a tty
- **GIVEN** a Linux x86_64 host with `libasound.so.2` already installed and `sudo` available
- **WHEN** the user runs `curl -fsSL <url>/install.sh | bash`
- **THEN** the script detects the target triple `x86_64-unknown-linux-gnu`
- **AND** the script queries the GitHub Releases API for the project's releases
- **AND** the script presents the user with the most recent stable releases via a `/dev/tty` prompt with the latest as the default
- **AND** on accepting the default, the script downloads `sound-cabinet-<latest-tag>-x86_64-unknown-linux-gnu` and its `.sha256` from `releases/download/<tag>/`
- **AND** the script verifies the binary's SHA-256 against the downloaded `.sha256` file
- **AND** the script installs the binary to `/usr/local/bin/sound-cabinet` (using `sudo install -m 755`)
- **AND** the script prints a final summary naming the installed version and path
- **AND** the script exits with status 0

#### Scenario: Default invocation when piped (no tty available)
- **GIVEN** a Linux host where stdin is the script content (the `curl | bash` case)
- **WHEN** the script reaches the version-selection step
- **THEN** the script detects the absence of `/dev/tty` (or its inaccessibility)
- **AND** the script proceeds with the latest stable release as if `--non-interactive` had been passed
- **AND** the script does NOT block waiting for user input

#### Scenario: Strict mode catches errors
- **WHEN** any command in the script fails (non-zero exit, unbound variable, or pipe failure)
- **THEN** the `ERR` trap fires
- **AND** an error message of the form `ERROR at line <N> (step: <step-name>)` is written to stderr
- **AND** the script exits with a non-zero status

### Requirement: Architecture detection and supported-triple mapping

The script SHALL detect the host OS and architecture via `uname -s` and `uname -m`, and SHALL map them to exactly one of four supported Rust target triples reachable through the bash installer:

| `uname -s` | `uname -m` | Target triple |
|---|---|---|
| `Linux` | `x86_64` (or `amd64`) | `x86_64-unknown-linux-gnu` |
| `Linux` | `aarch64` (or `arm64`) | `aarch64-unknown-linux-gnu` |
| `Darwin` | `arm64` (or `aarch64`) | `aarch64-apple-darwin` |
| `Darwin` | `x86_64` (or `amd64`) | `x86_64-apple-darwin` |

Any other combination — `Linux` + `armv7l` / `i686` / `riscv64` / `ppc64le` / `s390x`, all `*BSD`, `MINGW*`, etc. — SHALL produce a clear "no pre-built binary for your platform via this installer" error referencing the GitHub Releases page (in case a binary exists for manual download) and the Build-from-source instructions in the README, and SHALL exit non-zero before any network call.

The `x86_64-pc-windows-msvc` target produced by the release pipeline is NOT reachable through this bash installer — the script does not run on native Windows. Windows users use the README's manual-download path with the `.zip` archive.

#### Scenario: Linux ARM64 maps to aarch64-unknown-linux-gnu
- **GIVEN** `uname -s` returns `Linux` and `uname -m` returns `aarch64`
- **WHEN** the script runs
- **THEN** the target triple is set to `aarch64-unknown-linux-gnu`

#### Scenario: macOS Apple Silicon maps to aarch64-apple-darwin
- **GIVEN** `uname -s` returns `Darwin` and `uname -m` returns `arm64`
- **WHEN** the script runs
- **THEN** the target triple is set to `aarch64-apple-darwin`

#### Scenario: Intel Mac maps to x86_64-apple-darwin
- **GIVEN** `uname -s` returns `Darwin` and `uname -m` returns `x86_64`
- **WHEN** the script runs
- **THEN** the target triple is set to `x86_64-apple-darwin`
- **AND** the script proceeds with the normal install flow

#### Scenario: Unknown architecture is not supported
- **GIVEN** `uname -m` returns a value not in the supported set (e.g. `armv7l`, `ppc64le`, `s390x`)
- **WHEN** the script runs
- **THEN** the script prints an error naming the detected platform
- **AND** the message points the user at both the GitHub Releases page (for manual download if a compatible binary exists) and the Build-from-source instructions in the README
- **AND** the script exits non-zero before making any HTTP request

### Requirement: Version selection wizard

The script SHALL resolve a version tag to install via the following precedence:

1. If `--version <tag>` was passed, the value is used verbatim. The script SHALL validate the tag exists in the released set (querying the API for confirmation) and SHALL refuse with a clear error if the tag is not found.
2. Else the script fetches the releases list from the GitHub Releases API (`GET <api-base-url>/repos/rbeverly/sound-cabinet/releases`).
3. The script filters the returned tags: by default, only tags matching `^v[0-9]+\.[0-9]+\.[0-9]+$` AND whose `prerelease` flag is `false` are kept. When `--prerelease` is passed, all tags are kept.
4. If a controlling tty is available AND `--non-interactive` / `-y` is not set: the top 5 filtered tags are presented as a numbered list with their publish dates, and the user is prompted via `/dev/tty` to choose by number, by typing a tag explicitly, or by accepting the default (`1`, the most recent).
5. Otherwise (no tty, or `--non-interactive`): the first filtered tag is used as the default.

The version-selection wizard SHALL NEVER prompt for anything besides the version choice — there are no other configurable post-install settings in sound-cabinet. The default SHALL always be a stable release; pre-releases SHALL require explicit opt-in via the flag or by typing the pre-release tag.

#### Scenario: Interactive default picks the latest stable
- **GIVEN** a tty is available and `--non-interactive` is not set
- **WHEN** the user presses Enter at the version prompt without typing anything
- **THEN** the script uses the first (most recent) tag from the filtered list — the latest stable release

#### Scenario: Interactive user picks by number
- **GIVEN** the wizard prompt has been printed
- **WHEN** the user types `3<enter>` at the prompt
- **THEN** the script uses the third tag in the listed top 5

#### Scenario: Interactive user types an explicit tag
- **GIVEN** the wizard prompt has been printed
- **WHEN** the user types `v0.3.0<enter>` (a tag that exists but is not in the top 5)
- **THEN** the script uses `v0.3.0` after validating it exists in the full filtered list

#### Scenario: Explicit invalid tag is rejected
- **GIVEN** the wizard prompt has been printed
- **WHEN** the user types `v9.9.9<enter>` (a tag that does not exist)
- **THEN** the script prints an error naming the unknown tag
- **AND** the script exits non-zero (no retry loop — keep the wizard simple; the user can re-run)

#### Scenario: --version flag bypasses the wizard
- **WHEN** the user passes `--version v0.3.0`
- **THEN** the script does NOT present the wizard prompt
- **AND** the script validates `v0.3.0` exists in the released set and proceeds to download

#### Scenario: --prerelease includes pre-release tags
- **GIVEN** the published releases include both stable and pre-release tags
- **WHEN** the user runs `--prerelease`
- **THEN** the wizard's filtered list includes pre-release tags (those whose `prerelease` flag is `true` and/or whose name has a dash suffix)
- **AND** the default is still the most recently published tag in that list (which may be a pre-release)

### Requirement: Download and SHA-256 verification

For the resolved (`<tag>`, `<triple>`) pair, the script SHALL:

1. Download two files into a `mktemp -d` workspace:
   - The archive: `<releases-base-url>/<tag>/sound-cabinet-<tag>-<triple>.tar.gz`
   - The checksum: `<releases-base-url>/<tag>/sound-cabinet-<tag>-<triple>.tar.gz.sha256`
   Both downloads SHALL use `curl -fSL --proto =https --tlsv1.2` so HTTPS is enforced and a non-2xx response aborts the script.
2. Run `sha256sum -c <archive-name>.sha256` (Linux) or `shasum -a 256 -c <archive-name>.sha256` (macOS) against the downloaded `.sha256` file. The script SHALL select the available tool at startup; if neither is present, the script SHALL abort with a clear "no sha256 verifier available" message before downloading anything.
3. Extract the archive: `tar -xzf <archive-name> -C <tmpdir>` yields a single `sound-cabinet` file at the workspace root. The script SHALL verify the extracted binary exists at `<tmpdir>/sound-cabinet` before proceeding to install; if missing, the script SHALL abort with a clear "extracted archive did not contain sound-cabinet" message.

On verification failure (sha256 mismatch), the script SHALL print both the computed digest and the expected digest, SHALL leave the workspace directory intact (NOT delete it), and SHALL exit non-zero with a message naming the workspace path so an operator can inspect the corrupted files. On any failure, the script SHALL NOT proceed to install.

The subsequent install step (covered by the install-path-resolution requirement) operates on the extracted `sound-cabinet` binary, not the archive.

#### Scenario: Successful download, verify, and extract
- **GIVEN** the GitHub Releases CDN serves the archive and the matching checksum
- **WHEN** the script downloads both files
- **THEN** `sha256sum -c <basename>.sha256` (or `shasum -a 256 -c` on macOS) succeeds
- **AND** `tar -xzf` produces a single `sound-cabinet` file at the workspace root
- **AND** the script proceeds to the install step

#### Scenario: 404 on archive URL aborts
- **WHEN** the archive URL returns a 404 (or any non-2xx)
- **THEN** curl exits non-zero
- **AND** the ERR trap fires with the download step in the message
- **AND** the script exits non-zero

#### Scenario: 404 on checksum URL aborts
- **WHEN** the archive URL succeeds but the `.sha256` URL returns a 404
- **THEN** the script aborts BEFORE proceeding to verification
- **AND** the script exits non-zero with a clear message

#### Scenario: Checksum mismatch preserves the workspace for inspection
- **GIVEN** the downloaded archive's SHA-256 does not match the contents of the downloaded `.sha256` file
- **WHEN** the script runs the verification step
- **THEN** the script prints both the computed and expected digests
- **AND** the script prints the absolute path to the temp workspace
- **AND** the script does NOT delete the temp workspace
- **AND** the script exits non-zero
- **AND** the install step does NOT run

#### Scenario: Neither sha256sum nor shasum available
- **GIVEN** a host where `command -v sha256sum` and `command -v shasum` both fail
- **WHEN** the script runs
- **THEN** the script aborts BEFORE downloading
- **AND** prints a "no sha256 verifier available; install coreutils (Linux) or use macOS" message
- **AND** exits non-zero

#### Scenario: Archive missing sound-cabinet binary aborts
- **GIVEN** an archive whose contents do not include a `sound-cabinet` file at the root
- **WHEN** the script extracts the archive
- **THEN** the script aborts with a clear "extracted archive did not contain sound-cabinet" message
- **AND** the install step does NOT run

### Requirement: Install path resolution

The script SHALL resolve the install directory via the following precedence:

1. `--prefix <path>` if passed → use that path verbatim.
2. `--user` if passed → use `${HOME}/.local/bin`.
3. Else, if `sudo` is available on `PATH` OR the current user is root → use `/usr/local/bin`.
4. Else (no sudo, not root) → fall back to `${HOME}/.local/bin` with a printed notice explaining the fallback.

When the install path is `/usr/local/bin` and the current user is not root, the script SHALL wrap the install command with `sudo`. The final installed file SHALL be `<install-path>/sound-cabinet` (no version suffix), with mode 0755, owned by the user/group appropriate for the install path. The script SHALL use `install -m 755 <src> <dst>` so permissions are set atomically (`install` creates the file with the requested mode rather than `cp`+`chmod`).

When the resolved install path is `${HOME}/.local/bin` (either explicitly or via fallback) AND that directory is not already in the user's `$PATH`, the script SHALL print PATH-setup snippets for bash, zsh, and fish so the user can append the appropriate line to their shell rc file. The script SHALL NOT modify the user's rc files itself.

#### Scenario: Default install to /usr/local/bin with sudo
- **GIVEN** sudo is available on PATH
- **WHEN** the script reaches the install step with no `--prefix` or `--user`
- **THEN** the script invokes `sudo install -m 755 <tmpdir>/<binary> /usr/local/bin/sound-cabinet`

#### Scenario: --user installs to ~/.local/bin without sudo
- **WHEN** the user passes `--user`
- **THEN** the script invokes `install -m 755 <tmpdir>/<binary> $HOME/.local/bin/sound-cabinet` (no sudo)
- **AND** if `$HOME/.local/bin` is not in `$PATH`, the script prints PATH-setup snippets for bash, zsh, and fish

#### Scenario: Fallback to ~/.local/bin when sudo unavailable
- **GIVEN** sudo is not on PATH AND the current user is not root
- **WHEN** the script reaches the install-path resolution step
- **THEN** the script prints a notice that sudo is unavailable
- **AND** the script proceeds with `$HOME/.local/bin` as if `--user` had been passed

#### Scenario: --prefix overrides everything
- **WHEN** the user passes `--prefix /opt/sound-cabinet/bin`
- **THEN** the script installs to `/opt/sound-cabinet/bin/sound-cabinet` regardless of `--user`, sudo availability, or root status
- **AND** the script wraps with sudo only if the target directory requires elevated permissions

### Requirement: Linux-only ALSA presence advisory

On Linux, after a successful install, the script SHALL check whether the ALSA runtime library is present (`ldconfig -p | grep -q 'libasound\.so\.2'`). If absent, the script SHALL print an advisory naming the appropriate install command for the detected distro family:

- Debian / Ubuntu / Linux Mint / Pop!_OS (or any distro where `/etc/os-release`'s `ID` is `debian`/`ubuntu` or `ID_LIKE` contains `debian`) → `sudo apt-get install -y libasound2`
- Fedora / RHEL / CentOS / Rocky / AlmaLinux → `sudo dnf install -y alsa-lib`
- Arch / Manjaro / EndeavourOS → `sudo pacman -S --noconfirm alsa-lib`
- Distro not recognized → generic "install libasound2 (Debian/Ubuntu) or alsa-lib (Fedora/Arch) via your package manager"

The script SHALL NOT attempt to install ALSA automatically. The script SHALL NOT abort on missing ALSA — the binary is still installed; the advisory is printed and the script exits 0. On macOS, no ALSA check runs (CoreAudio is built in).

#### Scenario: Linux with ALSA present
- **GIVEN** `ldconfig -p` lists `libasound.so.2`
- **WHEN** the script runs the ALSA check
- **THEN** no advisory is printed
- **AND** the script proceeds silently

#### Scenario: Debian-family Linux without ALSA prints apt advisory
- **GIVEN** `ldconfig -p` does NOT list `libasound.so.2` AND `/etc/os-release` has `ID=debian` (or `ID_LIKE=debian`)
- **WHEN** the script runs the ALSA check
- **THEN** the script prints `! ALSA runtime library not detected. Run: sudo apt-get install -y libasound2`
- **AND** the script does NOT abort (exit 0 after install completes)

#### Scenario: Fedora-family without ALSA prints dnf advisory
- **GIVEN** missing `libasound.so.2` AND `/etc/os-release` has `ID=fedora`
- **WHEN** the script runs the ALSA check
- **THEN** the advisory recommends `sudo dnf install -y alsa-lib`

#### Scenario: macOS skips the ALSA check entirely
- **GIVEN** `uname -s` returns `Darwin`
- **WHEN** the script reaches what would be the ALSA-check step
- **THEN** the step is skipped (no `ldconfig` invocation, no advisory)

### Requirement: --dry-run prints actions without executing them

The `--dry-run` flag SHALL prevent every destructive operation (download to disk, install, sudo invocation, file replace) and SHALL instead print each intended action prefixed with `[dry-run]`. The script SHALL still execute non-destructive operations: architecture detection, the API call to fetch releases, the version-selection prompt (so the operator can see the full flow), and the ALSA-presence check. After dry-run completes, the filesystem outside the script's mktemp workspace SHALL be unchanged.

#### Scenario: Dry-run leaves no install artifact
- **GIVEN** a clean host with no `sound-cabinet` binary installed
- **WHEN** the user runs `bash install.sh --dry-run --non-interactive`
- **THEN** the script prints `[dry-run]` lines for the download, verify, and install steps
- **AND** no file is written to `/usr/local/bin`, `~/.local/bin`, or any other persistent location
- **AND** the script exits 0

#### Scenario: Dry-run still exercises the API call and prompt
- **WHEN** the user runs `--dry-run` with a tty available
- **THEN** the script DOES fetch the releases list (read-only API call)
- **AND** the script DOES present the version-selection prompt (the operator wants to see the actual UX)
- **AND** the subsequent download step is `[dry-run]`-only

### Requirement: --non-interactive / -y mode

The `--non-interactive` flag (and its short alias `-y`) SHALL cause the script to proceed with all defaults: latest stable version, default install path (`/usr/local/bin` with sudo, else `~/.local/bin`), no prompts. The script SHALL NOT read from `/dev/tty` in non-interactive mode even if a tty is available. The flag is intended for CI, IaC, and automated re-install scenarios.

#### Scenario: Non-interactive in CI
- **GIVEN** a CI environment that runs `bash install.sh --non-interactive` (or `-y`)
- **WHEN** the script runs
- **THEN** no `/dev/tty` reads occur
- **AND** the latest stable version is selected
- **AND** the default install path is used

#### Scenario: -y is an alias for --non-interactive
- **WHEN** the user passes `-y`
- **THEN** behavior is identical to `--non-interactive`

### Requirement: --api-base-url and --releases-base-url enable harness testability

The script SHALL accept `--api-base-url <url>` (default `https://api.github.com`) and `--releases-base-url <url>` (default `https://github.com/rbeverly/sound-cabinet/releases/download`) flags. All API queries and CDN downloads SHALL use these base URLs. The flags exist primarily so a test harness can substitute a local fixture server (or, in the case of the bundled bash test harness, a `curl` shim placed first on PATH).

The script SHALL NOT hard-code any GitHub URL. Every URL is derived from one of these two base variables.

#### Scenario: Custom API base routes through the test shim
- **GIVEN** the test harness sets `PATH="<fixtures>:$PATH"` (with a curl shim in fixtures) and passes `--api-base-url http://localhost.test`
- **WHEN** the script runs
- **THEN** the shim's curl receives URLs starting with `http://localhost.test/repos/...`
- **AND** the script does not attempt to reach `api.github.com`

### Requirement: Harness tests cover every documented scenario

The repository SHALL include a test harness under `tests/install/` consisting of:

- `tests/install/run.sh` — top-level runner that executes every `tests/install/*.test.sh` file and reports pass/fail counts.
- `tests/install/lib.sh` — shared helpers (`setup_test`, `teardown_test`, `run_install`, `assert_*`).
- `tests/install/fixtures/curl` — a bash shim that intercepts curl calls, dispatches on URL pattern (including `.tar.gz` and `.sha256` archive URLs, plus HEAD-request handling for tag-validation paths), and writes fixture content to the requested `-o` path.
- `tests/install/fixtures/uname` — a bash shim that returns `${MOCK_UNAME_S}` and `${MOCK_UNAME_M}`.
- `tests/install/fixtures/sudo` — a bash shim that records its argv and then `exec`s the wrapped command (so sudo calls are observable without elevating privileges).
- `tests/install/fixtures/tar` — a bash shim that "extracts" a fixture archive by writing a known-content `sound-cabinet` binary to the `-C` directory, so tests don't need real gzip-formatted fixtures.
- `tests/install/fixtures/releases.json` — a hand-crafted Releases API response containing at least 4 stable and 3 pre-release entries with deterministic timestamps.
- `tests/install/fixtures/archives/` — marker files named per the asset-naming convention (`sound-cabinet-<tag>-<triple>.tar.gz`) for each (tag, triple) combination tested, plus matching `.sha256` files, plus at least one deliberately-corrupted `.sha256` for the mismatch test.

Every documented behavior in this spec SHALL have at least one corresponding `*.test.sh` file, including a `darwin_x86_64_default_install.test.sh` exercising the Intel Mac happy path. `bash tests/install/run.sh` SHALL exit 0 with all tests passing.

#### Scenario: Every spec scenario is testable in the sandbox
- **WHEN** the implementing agent runs `bash tests/install/run.sh` in the sandbox
- **THEN** every test file executes without network access
- **AND** every test file executes without writing to `/usr/local/bin`, `$HOME` outside `$TMPDIR`, or any persistent host location
- **AND** all tests pass

#### Scenario: The harness uses PATH shims, not mock libraries
- **WHEN** an auditor inspects `tests/install/lib.sh`
- **THEN** the harness sets `PATH="$REPO_ROOT/tests/install/fixtures:$PATH"` before running `install.sh`
- **AND** no test mocks `install.sh`'s internal functions directly — interception happens at the tool boundary (`curl`, `uname`, `sudo`, `tar`)

#### Scenario: Intel Mac happy path is tested
- **WHEN** the test runner reaches `darwin_x86_64_default_install.test.sh`
- **THEN** the test sets `MOCK_UNAME_S=Darwin MOCK_UNAME_M=x86_64` and runs `install.sh --dry-run --non-interactive ...`
- **AND** the test asserts the resolved triple is `x86_64-apple-darwin`
- **AND** the test asserts a successful exit code

### Requirement: README quick-install section

The README SHALL be updated so that the `## Install` section opens with a **Quick install** subsection containing the curl one-liner, a one-sentence description of what the installer does, and a reference to the `--help` flag for the full flag list. The pre-existing manual binary-install instructions SHALL be demoted to a smaller **Manual install** subsection pointing readers at the GitHub Releases page. The **Build from source** subsection SHALL remain unchanged.

#### Scenario: Quick install is the first install instruction
- **WHEN** a reader opens README and scrolls to the Install section
- **THEN** the Quick install subsection appears first
- **AND** the curl one-liner is the first install command shown

#### Scenario: Manual fallback still documented
- **WHEN** a reader looks past Quick install
- **THEN** a Manual install subsection explains how to download from the GitHub Releases page directly
- **AND** the asset naming convention (`sound-cabinet-<tag>-<triple>`) is named so the reader can construct the URL by hand

