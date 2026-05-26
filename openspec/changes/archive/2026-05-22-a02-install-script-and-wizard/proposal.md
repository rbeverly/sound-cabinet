## Why

The current install story is: `git clone`, install Rust (if not already there), install `libasound2-dev` (Linux only, distro-dependent command), `cargo build --release`, copy the binary somewhere on PATH. That works for Rust developers; it's a wall for anyone else. The README's "Pre-built binaries" section helps but requires the user to construct the right URL by hand for their architecture, fall back to a manual `tar xz` + `sudo mv`, and remember to install ALSA on Linux.

The fix is the standard table-stakes installer: `curl -fsSL <url>/install.sh | bash`. Sound-cabinet is small enough — single binary, no daemon, no config file, no system user, no database — that the whole installer fits in a single, testable bash script. Trying to push wizard logic into a Rust subcommand (the architecture cicd-impl-agents took for autocoder) is overengineering here: there's nothing to configure post-install. The only interactive choices are (1) which version and (2) which install location, both of which a short bash script can do cleanly with `/dev/tty`-based prompts and sensible defaults.

The wizard is constrained by a real-world tension: when invoked as `curl … | bash`, stdin is the script body, not the terminal — so `read` doesn't reach the user. The installer reads from `/dev/tty` directly for prompts and falls back to defaults (latest stable version, install to `/usr/local/bin` via sudo) when no controlling tty is present (CI environments, IaC, intentional `--non-interactive` mode).

## What Changes

- **NEW**: `install.sh` at the repo root. Pure bash, target ~150-200 lines including comments and help text. Strict mode (`set -euo pipefail`), `trap` on `ERR` reporting the last labeled step, no exotic dependencies (uses tools that ship on every standard macOS and Debian install: `curl`, `tar`, `uname`, `mktemp`, `install`, `sha256sum` or `shasum`, `ldconfig` on Linux, `id`, `sudo` if available).
- **NEW**: harness tests under `tests/install/` — bash-based fixture tests that prepend a mock PATH and run `install.sh` with `--dry-run` against synthetic GitHub Releases API responses. The autocoder sandbox CAN execute these (no real network, no real sudo, no real install to /usr/local/bin). This is the testability story.
- **MODIFIED**: README — replace the current "Install → Pre-built binaries" subsection with a "Quick install" section using the one-liner; demote the existing manual binary instructions to a "Manual install" subsection further down; keep "Build from source" unchanged.
- **DEPENDS ON**: `a01-release-pipeline-github-actions` change being merged AND at least one stable tag pushed. Without published binaries, the installer has nothing to download. The agent SHOULD NOT attempt to push tags or trigger the release workflow — that is the maintainer's manual step.

### Installer behavior

The script's responsibilities in order:

1. **Parse flags.** Accepted flags: `--version vX.Y.Z` (pin a specific tag), `--prerelease` (include pre-release tags in the version list), `--prefix <path>` (override the install directory), `--user` (shortcut for `--prefix ~/.local/bin`, no sudo), `--non-interactive` / `-y` (skip all prompts, use defaults), `--dry-run` (print steps without executing destructive operations), `--api-base-url <url>` (override the GitHub Releases API base — used by tests to inject fixture responses), `--help`.
2. **Detect OS and architecture.** Map `uname -s` + `uname -m` to one of the three supported Rust target triples:
   - `Linux` + `x86_64` → `x86_64-unknown-linux-gnu`
   - `Linux` + `aarch64`/`arm64` → `aarch64-unknown-linux-gnu`
   - `Darwin` + `arm64`/`aarch64` → `aarch64-apple-darwin`
   - Anything else (including `Darwin` + `x86_64`, all `*BSD`, all Windows-via-MSYS) → exit non-zero with a clear "no pre-built binary for your platform; build from source per README" message. Intel Macs are explicitly out of scope for this installer; the Cargo.lock / Cargo.toml haven't been built for them in the new release pipeline.
3. **Resolve the version to install.**
   - If `--version vX.Y.Z` was passed, use that tag verbatim. No API call.
   - Else: query `GET <api-base-url>/repos/rbeverly/sound-cabinet/releases` (the full list, so we can filter and present multiple choices). Parse JSON without `jq` to keep dependencies minimal — use a small `grep`+`sed` extractor (acceptable for this single, well-defined JSON shape).
   - Filter tags: by default, only those matching `^v[0-9]+\.[0-9]+\.[0-9]+$` (stable). With `--prerelease`, include all tags. Sort by GitHub's published-at order (most recent first).
   - If a controlling tty is available and `--non-interactive` is not set:
     - Print the top 5 candidate tags as a numbered list with their publish date.
     - Print `Install [1] (latest), [2], ... or type a tag like vX.Y.Z. Default [1]:` and read one line from `/dev/tty`.
     - Empty input → option 1 (latest). Numeric input → that position in the list. Other input → treat as a literal tag name (validated against the list; if not found, refuse).
   - If no tty OR `--non-interactive` is set: use position 1 (the most recent stable tag).
4. **Determine the install path.**
   - `--prefix <path>` wins if set.
   - Else `--user` → `${HOME}/.local/bin`.
   - Else: default to `/usr/local/bin` with sudo elevation (the script re-execs the install step with `sudo install -m 755`). If `sudo` is not on PATH and the current user isn't root, fall back to `${HOME}/.local/bin` with a printed notice.
5. **Linux only: ALSA presence check.**
   - Run `ldconfig -p 2>/dev/null | grep -q libasound\\.so\\.2`.
   - On match: continue silently.
   - On miss: print a clear advisory naming the install command for the detected distro (`apt-get install libasound2` for Debian/Ubuntu, `dnf install alsa-lib` for Fedora/RHEL, `pacman -S alsa-lib` for Arch). Detection uses `/etc/os-release`'s `ID` field (or `ID_LIKE` as fallback). Distro not detected → print a generic "install libasound2 / alsa-lib via your package manager" message. The installer continues; the binary is installed but cannot run until the user installs ALSA. Exit code remains 0 (the install itself succeeded).
6. **Download.** `curl -fSL --proto =https --tlsv1.2 -o <tmpdir>/<binary-name> <releases-cdn>/sound-cabinet-<tag>-<triple>` and the same for the `.sha256` URL. `<releases-cdn>` defaults to `https://github.com/rbeverly/sound-cabinet/releases/download/<tag>/` but is also overridable via `--releases-base-url` for testing. Refuse to proceed on non-2xx from either URL (curl `-f` does this automatically). Use `mktemp -d` for the workspace; clean up on success, preserve on failure for debugging.
7. **Verify.** `cd <tmpdir> && sha256sum -c <basename>.sha256` (Linux) or `shasum -a 256 -c <basename>.sha256` (macOS). On mismatch: print the computed and expected digests side-by-side, do NOT delete `<tmpdir>`, exit non-zero with a clear "checksum verification failed; the temp dir at <path> has the files for inspection" message.
8. **Install.** `install -m 755 <tmpdir>/<binary-name> <install-path>/sound-cabinet` (wrapped in `sudo` when target is system-wide). Replaces any prior binary at that path.
9. **PATH hint** (only on the `--user` / fallback path). If `${HOME}/.local/bin` is not in `$PATH`, print the right one-line snippet for each common shell (bash, zsh, fish) so the user can append it to their rc file. Do NOT modify the user's rc file; just print the lines.
10. **Final summary.** Print:
    - The installed version.
    - The full install path of the binary.
    - The first command to try (`sound-cabinet --help`).
    - On Linux with missing ALSA: a repeat of the ALSA hint.
    - On `--user` path with `~/.local/bin` not on `$PATH`: the PATH-addition snippet.

### Dry-run mode

`--dry-run` SHALL prevent every destructive operation (file write, file move, `sudo` call, network download) and instead print each intended step prefixed with `[dry-run]`. The mode SHALL exercise every code path that runs in a real install up to the point of action: arch detection, API call (still real), checksum file (still downloaded, still verified, just not installed), prompt handling. This makes `--dry-run` both an operator-facing review tool AND the primary handle the test harness uses to assert behavior without touching the host.

### Test harness

Under `tests/install/`:

- `tests/install/fixtures/curl` — a bash shim placed first on PATH during tests. It dispatches based on the URL argument:
  - `GET .../releases` → cats a fixture JSON file from `tests/install/fixtures/releases.json`
  - `GET .../sound-cabinet-vX.Y.Z-<triple>` → writes a known-content placeholder binary to the `-o` target
  - `GET .../sound-cabinet-vX.Y.Z-<triple>.sha256` → writes a sha256 file matching that placeholder
  - Unknown URL → exits 22 (matches real curl's behavior for `-f` on 404)
- `tests/install/fixtures/uname` — a shim that prints whatever `MOCK_UNAME_S` / `MOCK_UNAME_M` env vars say. Used to test each (OS, arch) combination on a single Linux runner.
- `tests/install/fixtures/sudo` — a shim that records its arguments to a file in `$TMPDIR` and then `exec`s the rest of its arguments (so sudo is observed but not actually run). Used to assert "sudo was invoked with the right install command."
- Each test is a `*.test.sh` file that sets `PATH="tests/install/fixtures:$PATH"`, sets the relevant `MOCK_UNAME_*` vars, runs `install.sh --dry-run …`, and asserts on the output / exit code / recorded sudo args.
- Test scenarios (one file each):
  - `linux_x86_64_default_install.test.sh` — happy path, latest stable, /usr/local/bin via sudo
  - `linux_aarch64_default_install.test.sh` — same but ARM64
  - `darwin_aarch64_default_install.test.sh` — macOS happy path (uses `shasum` not `sha256sum`)
  - `darwin_x86_64_unsupported.test.sh` — exits non-zero with the right message
  - `version_flag_pins_tag.test.sh` — `--version v0.5.0` is used verbatim, no API call
  - `prerelease_flag_includes_prereleases.test.sh` — `--prerelease` surfaces dash-suffix tags
  - `user_flag_installs_to_home.test.sh` — `--user` writes to ~/.local/bin, no sudo
  - `non_interactive_picks_latest_stable.test.sh` — `--non-interactive` skips prompts, uses position 1
  - `no_tty_falls_back_to_defaults.test.sh` — when /dev/tty isn't available the script proceeds as if `--non-interactive`
  - `sha256_mismatch_aborts.test.sh` — corrupted fixture, script exits non-zero, tmpdir preserved
  - `404_on_binary_url_aborts.test.sh` — curl returns 22 / 404, script exits non-zero
  - `404_on_checksum_url_aborts.test.sh` — same for the .sha256 URL
  - `alsa_missing_warns_but_continues.test.sh` — Linux, no libasound2, exit 0 with ALSA hint in output
  - `alsa_present_no_warning.test.sh` — Linux, libasound2 found, no ALSA hint in output
  - `dry_run_no_install.test.sh` — `--dry-run` leaves the filesystem unchanged
  - `unknown_arch_errors_with_build_from_source_hint.test.sh` — arm32, ppc64le, etc.
  - `help_flag_prints_usage.test.sh` — `--help` exits 0 with usage text

A small test runner script `tests/install/run.sh` iterates `*.test.sh` files and reports pass/fail counts. Each `.test.sh` is self-contained; failure is non-zero exit. No bats, no shellcheck assumed.

### What's NOT in this change

- No Rust subcommand. Sound-cabinet is too small to justify a `sound-cabinet install` subcommand; the wizard fits cleanly in bash.
- No Windows support in the installer. The README's Windows manual-install section remains as-is. (The release workflow also skips Windows.)
- No code signing. macOS users will get a Gatekeeper prompt on first run; the README documents the `xattr -d com.apple.quarantine` workaround.
- No automatic ALSA install. Detected-and-warned only; the user runs the package manager themselves. Auto-installing packages requires sudo and is too invasive for a tool installer.
- No `install.sh` self-update mechanism. The user re-runs the one-liner to update; that's the whole upgrade story.

## Capabilities

### New Capabilities

- `install-script`: the curl-runnable bash installer covering arch detection, version selection (interactive when a tty is available, default-driven otherwise), download + sha256 verification, install-path resolution, and Linux-specific ALSA-presence advisory.

### Modified Capabilities

(None — this change adds the installer; existing specs are untouched.)

## Impact

- **Affected specs**: `install-script` (new capability, multiple ADDED requirements). No existing specs modified.
- **Affected code**:
  - `install.sh` — new file at repo root, ~150-200 lines.
  - `tests/install/` — new directory: `run.sh`, `fixtures/` (curl + uname + sudo shims, `releases.json`), and one `*.test.sh` per scenario above.
  - `README.md` — replace "Pre-built binaries" with "Quick install" section; demote and trim the manual binary instructions.
- **Operator-visible behavior**: a working `curl -fsSL <url>/install.sh | bash` one-liner. The interactive prompt lists the most recent stable releases; defaults match user expectations.
- **Dependencies**: requires the `a01-release-pipeline-github-actions` change to be merged AND at least one stable tag (`vX.Y.Z`, no dash suffix) to be published. The installer's tests use fixture JSON, so the test suite does NOT depend on a real release — but the README's published one-liner does.
- **Breaking**: no. The existing `cargo build --release` path remains documented.
- **Acceptance**:
  - `bash -n install.sh` clean.
  - `bash tests/install/run.sh` passes all listed scenarios.
  - `openspec validate install-script-and-wizard --strict` passes.
  - The implementing agent CAN run all of the above in its sandbox without network access (the curl shim handles that). The agent CANNOT verify the one-liner works in production — that's a manual maintainer step after the change merges.

## Constraints visible to the implementing agent

- The agent's sandbox cannot reach GitHub. The test harness uses a `curl` shim that returns fixture data; the production code path is reachable only by structural inspection and the dry-run mode.
- The agent SHOULD NOT assume `shellcheck` is installed. If it's on PATH, the agent SHOULD run `shellcheck install.sh` and fail on errors; if not, the agent SHOULD print one `WARN: shellcheck not available in sandbox — skipping lint` line and proceed.
- The agent SHOULD NOT introduce `bats`, `bash-it`, `expect`, or any other test framework. The harness is plain `*.test.sh` files for the same reason `install.sh` itself is plain bash: it has to work in the sandbox without provisioning.
- The agent SHOULD NOT write to `/usr/local/bin`, `$HOME`, or anywhere else outside the workspace during tests. The harness sandboxes via `$PATH` shimming and `$TMPDIR` writes only; the `sudo` shim makes "what would have been a sudo invocation" observable without actually executing it.
- The agent SHOULD NOT push tags, trigger the release workflow, or attempt to publish to GitHub. The installer's `tests/install/` fixtures are the entire validation surface available in the sandbox.
- The agent CANNOT verify the installer works on macOS in its sandbox. Mac-specific behavior is tested via the `uname` shim and the `shasum`-vs-`sha256sum` selection in `install.sh`; the actual Mac binary install path is verified later by the maintainer running the one-liner on a real Mac.
