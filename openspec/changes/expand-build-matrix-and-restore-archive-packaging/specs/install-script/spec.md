# install-script Specification (delta)

## MODIFIED Requirements

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
