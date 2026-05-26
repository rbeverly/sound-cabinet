## 1. Bootstrap `install.sh`

- [x] 1.1 Create `install.sh` at the repo root. Set `#!/usr/bin/env bash` shebang. First non-comment line: `set -euo pipefail`. Define `IFS=$'\n\t'`. Add `trap 'echo "ERROR at line $LINENO (step: ${CURRENT_STEP:-unknown})" >&2' ERR`. Use a `CURRENT_STEP=...` assignment at the top of each major section so the trap is informative.
- [x] 1.2 Define top-level constants near the top of the file:
  - `REPO_OWNER="rbeverly"`
  - `REPO_NAME="sound-cabinet"`
  - `BINARY_NAME="sound-cabinet"`
  - `DEFAULT_API_BASE="https://api.github.com"`
  - `DEFAULT_RELEASES_BASE="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download"`
- [x] 1.3 Parse CLI flags. Implement a simple while-loop case-statement parser (no `getopts`, no `getopt`) for: `--version`, `--prerelease`, `--prefix`, `--user`, `--non-interactive` / `-y`, `--dry-run`, `--api-base-url`, `--releases-base-url`, `--help`. Validate that `--version` and `--prerelease` are not both passed (no need to filter to prereleases if a specific version is pinned). On `--help`, print a usage block and exit 0.
- [x] 1.4 Implement `detect_target_triple()`:
  - `os=$(uname -s)` — `Linux` or `Darwin` accepted; others error.
  - `arch=$(uname -m)` — `x86_64`/`amd64` → `x86_64`; `arm64`/`aarch64` → `aarch64`; others error.
  - Mapping table per the proposal. `Darwin` + `x86_64` errors with a clear "Intel Mac is out of scope; build from source per README" message.
  - On unsupported combo: print the message, exit 1.
- [x] 1.5 Implement `fetch_releases_json(api_base_url)`:
  - Single `curl -fsSL` call to `${api_base_url}/repos/${REPO_OWNER}/${REPO_NAME}/releases`.
  - Echo the JSON to stdout for the caller to capture.
  - Use `--proto =https --tlsv1.2` so a hostile DNS doesn't downgrade to plaintext.
- [x] 1.6 Implement `parse_tags_from_json(json, include_prereleases)`:
  - Use `grep -E` + `sed` to extract every `"tag_name": "..."` and `"prerelease": (true|false)` pair, walking them in order so each tag is paired with its prerelease flag. (The GitHub Releases API returns these together inside each release object; the order in the JSON is stable.)
  - Filter: when `include_prereleases=false`, drop entries with `"prerelease": true` AND drop entries whose tag does not match `^v[0-9]+\.[0-9]+\.[0-9]+$` (defense in depth — the prerelease flag is set by the workflow, but the regex is a second filter against malformed tags).
  - Output one tag per line in API-publish order (most recent first).
- [x] 1.7 Implement `select_version(parsed_tags_list, --version, --non-interactive)`:
  - If `--version` was set: validate the tag is in the parsed list (when `--prerelease` was also set; otherwise validate against the unfiltered list). Reject with a clear message if not found. Return the tag.
  - Else if no tty available (`! [ -e /dev/tty ]`) OR `--non-interactive` was set: return the first tag in the list (latest stable).
  - Else: print the top 5 tags as a numbered list (`  [1] v0.5.0 (2026-05-15)`, etc.); print `Choose a version to install. Default [1]: `; read one line from `/dev/tty`; interpret as described in proposal §3. Validate. Return the resolved tag.
- [x] 1.8 Implement `determine_install_path(--prefix, --user)`:
  - `--prefix` wins.
  - `--user` → `${HOME}/.local/bin`.
  - Else if `sudo` is available OR `id -u` returns 0 → `/usr/local/bin`.
  - Else → `${HOME}/.local/bin` with a printed notice ("sudo not available — installing to $HOME/.local/bin instead").
  - Return the chosen directory.
- [x] 1.9 Implement `linux_alsa_check()`:
  - Only runs when `$(uname -s)` is `Linux`.
  - `ldconfig -p 2>/dev/null | grep -q 'libasound\.so\.2'` → if present, return silently.
  - Else: read `/etc/os-release` (test `-r` first), extract `ID` and `ID_LIKE`. Map to install command:
    - `debian` / `ubuntu` / `ID_LIKE` contains `debian` → `sudo apt-get install -y libasound2`
    - `fedora` / `rhel` / `centos` / `ID_LIKE` contains `fedora` or `rhel` → `sudo dnf install -y alsa-lib`
    - `arch` / `manjaro` → `sudo pacman -S --noconfirm alsa-lib`
    - other / unknown → generic "install libasound2 (Debian/Ubuntu) or alsa-lib (Fedora/Arch) via your package manager"
  - Print the hint with a clear `! ALSA runtime library not detected. Run: <command>` heading. Set a flag `ALSA_HINT_NEEDED=1` so the final summary can repeat it. Return 0 (the install continues).
- [x] 1.10 Implement `download_and_verify(releases_base_url, tag, triple, tmpdir)`:
  - Construct `binary_url="${releases_base_url}/${tag}/${BINARY_NAME}-${tag}-${triple}"` and `checksum_url="${binary_url}.sha256"`.
  - `cd "$tmpdir"`. Two `curl -fSL --proto =https --tlsv1.2 -o <basename> <url>` calls. Refuse to proceed on non-2xx (curl's `-f` flag handles this).
  - Select checksum tool: `sha256sum` on Linux, `shasum -a 256` on macOS. Resolve once at script start; store in a `SHA256_VERIFY` array variable.
  - Run `${SHA256_VERIFY[@]} -c "${BINARY_NAME}-${tag}-${triple}.sha256"`. On mismatch: print computed-vs-expected, do NOT remove `tmpdir`, exit non-zero. On success: continue.
- [x] 1.11 Implement `install_binary(tmpdir, tag, triple, install_path)`:
  - Source file: `${tmpdir}/${BINARY_NAME}-${tag}-${triple}`.
  - Dest path: `${install_path}/${BINARY_NAME}` (no version suffix on installed name — running `sound-cabinet` is the goal).
  - When `install_path` is `/usr/local/bin` AND current user is not root: prefix with `sudo`.
  - Use `install -m 755 <src> <dst>` (creates the parent dir if missing; sets 755 perms atomically). Override with `mkdir -p <install_path>` first if `install` can't make the parent.
  - On `--dry-run`: print `[dry-run] sudo install -m 755 ${tmpdir}/<binary> <install_path>/<binary>` instead of executing.
- [x] 1.12 Implement `path_check_and_hint(install_path)`:
  - Only runs when `install_path` starts with `$HOME` (i.e. the user-prefix path).
  - Check whether `$install_path` is in `$PATH` (`case ":$PATH:" in *":$install_path:"*) found=1 ;; esac` style).
  - If not, print three shell-specific lines so the user can copy-paste one into their rc file:
    - bash: `echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc`
    - zsh: `echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc`
    - fish: `fish_add_path ~/.local/bin`
  - Set `PATH_HINT_NEEDED=1` so the final summary can repeat it.
- [x] 1.13 Final summary block. After the install completes:
  - Print `Installed sound-cabinet ${tag} to ${install_path}/sound-cabinet`.
  - Print `Try: sound-cabinet --help`.
  - If `ALSA_HINT_NEEDED=1`: re-print the ALSA install command.
  - If `PATH_HINT_NEEDED=1`: re-print the PATH-setup snippet for each shell.
- [x] 1.14 Implement `--dry-run` gating. Every destructive call site (curl downloads, install, sudo, file moves) checks a `DRY_RUN` flag and, when set, prints `[dry-run] <command>` instead of executing. The version-resolution API call IS still made in dry-run (it's read-only) so the rest of the pipeline can be exercised end-to-end.
- [x] 1.15 Length budget: aim for ≤ 200 lines including blank lines and comments, ≤ 250 hard ceiling. If the implementing agent exceeds the ceiling, that's a signal to split logic into helper functions and trim — not to add a Rust subcommand.

## 2. Test harness

- [x] 2.1 Create `tests/install/` directory.
- [x] 2.2 Create `tests/install/fixtures/curl` — bash script, `chmod +x`. Behavior:
  - Parse out the `-o <path>` flag and the trailing URL argument (last positional).
  - Dispatch on URL:
    - `*/repos/*/releases$` → `cat tests/install/fixtures/releases.json` to stdout (or `-o` path if set)
    - `*/sound-cabinet-*-*.sha256` → derive the binary name from the URL, look up the matching `<basename>.sha256` fixture under `tests/install/fixtures/binaries/`, copy to `-o` path
    - `*/sound-cabinet-*-*` (no `.sha256`) → look up the matching binary fixture file, copy to `-o` path
    - any other URL → exit 22 (mirrors `curl -f` 404 behavior)
  - Accept (and ignore) the curl flags actually used by `install.sh`: `-f`, `-s`, `-S`, `-L`, `--proto =https`, `--tlsv1.2`, `-o`.
- [x] 2.3 Create `tests/install/fixtures/uname` — bash script. If `$1 == "-s"` echo `${MOCK_UNAME_S:-Linux}`. If `$1 == "-m"` echo `${MOCK_UNAME_M:-x86_64}`. With no arg, echo `${MOCK_UNAME_S} ${MOCK_UNAME_M}`.
- [x] 2.4 Create `tests/install/fixtures/sudo` — bash script. `echo "sudo: $*" >> "${SUDO_LOG:-$TMPDIR/sudo.log}"`. Then `exec "$@"` so the wrapped command runs (but in `--dry-run` mode the wrapped command itself will be a no-op).
- [x] 2.5 Create `tests/install/fixtures/releases.json` — hand-crafted JSON mimicking the GitHub Releases API shape. Include at least 7 entries:
  - 4 stable (`v0.5.0`, `v0.4.0`, `v0.3.1`, `v0.3.0`) with `"prerelease": false`
  - 3 prerelease (`v0.6.0-rc1`, `v0.5.0-dev.2`, `v0.4.0-beta.1`) with `"prerelease": true`
  - Each entry has `"tag_name"`, `"prerelease"`, `"published_at"` fields with deterministic timestamps.
- [x] 2.6 Create `tests/install/fixtures/binaries/` directory containing:
  - `sound-cabinet-v0.5.0-x86_64-unknown-linux-gnu` — short text file with known content (e.g. `#!/usr/bin/env bash\necho 'fake sound-cabinet v0.5.0 linux x86_64'\n`)
  - `sound-cabinet-v0.5.0-x86_64-unknown-linux-gnu.sha256` — `<digest>  <binary-name>\n` matching the file above
  - Same for the other two triples
  - A `sound-cabinet-v0.5.0-x86_64-unknown-linux-gnu.bad-checksum.sha256` — wrong digest, used by the mismatch test
- [x] 2.7 Create `tests/install/lib.sh` — shared test helpers:
  - `setup_test()` — creates a temp workdir, points `$TMPDIR` there, sets `PATH="$REPO_ROOT/tests/install/fixtures:$PATH"`, sets `SUDO_LOG="$TMPDIR/sudo.log"`, exports `MOCK_UNAME_S`/`MOCK_UNAME_M`/test-specific env.
  - `teardown_test()` — `rm -rf` the workdir (skipped if the test failed, so the maintainer can inspect).
  - `assert_contains(haystack, needle)`, `assert_equals`, `assert_exit_code(expected, command...)`, `assert_file_exists`, `assert_file_missing`.
  - `run_install(args...)` — invokes `bash "$REPO_ROOT/install.sh" "$@" 2>&1` and stores stdout+stderr in `$TEST_OUTPUT` and exit code in `$TEST_EXIT_CODE`.
- [x] 2.8 Create `tests/install/run.sh` — runner. `cd "$REPO_ROOT"`. `for t in tests/install/*.test.sh; do bash "$t"; done`. Print `PASS: <t>` / `FAIL: <t>` per file. Track totals; exit 1 if any fail.
- [x] 2.9 Create each `*.test.sh` file per the proposal's test scenarios list. Each file sources `tests/install/lib.sh`, calls `setup_test`, runs `run_install` with the scenario's args + env vars, calls a series of asserts, calls `teardown_test`. Keep each file ≤ 30 lines for readability.

## 3. README update

- [x] 3.1 In `README.md`'s `## Install` section, REPLACE the existing **"Pre-built binaries"** subsection with a new **"Quick install"** subsection at the top of `## Install`. Content:
  - The curl one-liner: `curl -fsSL https://raw.githubusercontent.com/rbeverly/sound-cabinet/master/install.sh | bash`
  - A one-sentence note that it detects your platform, prompts for the version, downloads + sha256-verifies, and installs to `/usr/local/bin` (or `~/.local/bin` with `--user`).
  - A `--help` reference for the flag list (`--version`, `--prerelease`, `--prefix`, `--user`, `--non-interactive`, `--dry-run`).
- [x] 3.2 RETAIN the **"Build from source"** subsection unchanged. RETAIN platform dependencies (ALSA on Linux) but link it from the Quick Install section so users hitting the ALSA warning can find the fix easily.
- [x] 3.3 The existing `sudo mv sound-cabinet /usr/local/bin/` manual install block: trim it down to a single short paragraph titled "Manual install" pointing readers at the GitHub Releases page and the asset naming convention. Most users should use the one-liner; this section is for the air-gapped / no-curl case.

## 4. Spec delta

- [x] 4.1 Author the ADDED requirements in `specs/install-script/spec.md` per the spec authored alongside this proposal. Validate with `openspec validate install-script-and-wizard --strict`.

## 5. Verification

- [x] 5.1 `openspec validate install-script-and-wizard --strict` passes.
- [x] 5.2 `bash -n install.sh` passes (syntax check).
- [x] 5.3 `bash tests/install/run.sh` passes — every `*.test.sh` exits 0, the runner reports zero failures.
- [x] 5.4 The implementing agent SHOULD attempt `shellcheck install.sh tests/install/*.sh` if shellcheck is on PATH. If not, print one `WARN: shellcheck not available in sandbox — skipping lint` line and proceed. DO NOT install shellcheck as part of the change; DO NOT fail the implementation if it's absent.
- [x] 5.5 Structural sanity checks:
  - `install.sh` line count is ≤ 250 (per the proposal's budget).
  - `install.sh` starts with `#!/usr/bin/env bash`, `set -euo pipefail`, a trap on ERR.
  - No call to `useradd`, `systemctl`, `apt-get install` (without dry-run), or any system-mutating command other than `install` to a path the user requested.
  - `install.sh` does NOT call `curl` against `api.github.com` or `github.com` directly — both are derived from variables (`DEFAULT_API_BASE`, `DEFAULT_RELEASES_BASE`) that the `--api-base-url` / `--releases-base-url` flags override. This is what makes the test harness's curl shim viable.
- [x] 5.6 NOT in scope for this change's verification: running `install.sh` against the real GitHub Releases API, installing the binary to a real machine, validating Mac behavior on an actual Mac. Those are the maintainer's manual smoke-test steps after the change merges. The implementing agent SHOULD NOT push commits, tags, or releases as part of verifying the change.

## 6. README acceptance

- [x] 6.1 After the README edits, manually re-read the Install section top-to-bottom and verify: (a) the Quick install block appears first and is the obvious recommended path, (b) the manual fallback exists but is clearly secondary, (c) the Build from source section is unchanged, (d) the ALSA dependency note for Linux is linked from the Quick install block.
