## Why

The shipped `install-script-and-wizard` and `release-pipeline-github-actions` changes (archived 2026-05-22) were drafted before the existing 4-target release workflow was discovered. The resulting implementation regressed on capability that already existed, and one bash-strict-mode bug slipped through the autocoder's verification:

1. **Lost build targets**: the pre-existing `.github/workflows/release.yml` built five things — `aarch64-apple-darwin`, `x86_64-apple-darwin` (Intel Mac), `x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`. The implemented workflow builds only three (`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `aarch64-apple-darwin`), having dropped Intel Mac and Windows entirely. Adding `aarch64-unknown-linux-gnu` was the new target the maintainer asked for; the others were already there and should not have been removed.
2. **Wrong asset packaging**: the implementation produces bare ELF/Mach-O binaries (no extension) instead of the `.tar.gz`/`.zip` archives the original workflow produced and the README already documents. Bare binaries are technically usable but break the README's curl-and-tar instructions, surprise anyone with external automation built on the old asset names, and lose the `executable bit` guarantee on Windows (where the file needs an `.exe` extension).
3. **Bug**: `install.sh` line 200 expands `"${maybe_sudo[@]}"` from an empty array under `set -u`, which bash treats as an unbound variable. Two test scenarios (`user_flag_installs_to_home.test.sh`, `dry_run_no_install.test.sh`) fail on a non-sandbox host because of this. The autocoder's sandbox apparently masked the failure, but the bug ships in master right now.

This change fixes all three issues in one cohesive pass, since they're tightly coupled: restoring the matrix legs requires restoring the archive packaging (the Windows leg is `.zip`-only anyway), and the install-script needs the extract step in lockstep with the workflow's packaging change. Splitting would force a transient state where workflow and installer disagree on file format.

No new tagged releases have been published yet, so the asset-naming break is theoretical — there are no external consumers of the bare-binary URLs to grandfather in.

## What Changes

### Bash strict-mode bug

- **MODIFIED**: `install.sh:install_binary()`. Replace `"${maybe_sudo[@]}"` with the safe-empty-expansion idiom `${maybe_sudo[@]+"${maybe_sudo[@]}"}` (or restructure with an explicit conditional). Either form keeps `set -u` happy when the array is empty (the `--user` / `--prefix ~/.local/bin` case).
- **VERIFIED BY**: the existing `user_flag_installs_to_home.test.sh` and `dry_run_no_install.test.sh` tests, which currently fail and SHALL pass after the change.

### Build matrix expansion

- **MODIFIED**: `.github/workflows/release.yml` build matrix. Restore the two dropped legs:
  - `x86_64-apple-darwin` on `macos-latest` (native build)
  - `x86_64-pc-windows-msvc` on `windows-latest` (native build)
  These join the three legs already present (`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `aarch64-apple-darwin`), for a total of 5 matrix entries.
- The existing `test` gate, `aarch64-linux` cross-toolchain setup, and `publish` job structure stay as they are.

### Asset packaging: bare binary → archive

- **MODIFIED**: `.github/workflows/release.yml`. After the `cargo build --release` step:
  - **Unix legs** (Linux x86_64, Linux aarch64, macOS aarch64, macOS x86_64): replace the existing `cp` rename with `tar czf sound-cabinet-<tag>-<triple>.tar.gz -C target/<triple>/release sound-cabinet`. The archive contains exactly one file at the archive root: `sound-cabinet` (mode 0755). Strip the binary before tar on Linux (macOS strip is implicit).
  - **Windows leg** (new): `Compress-Archive` the `sound-cabinet.exe` from `target/<triple>/release/` into `sound-cabinet-<tag>-x86_64-pc-windows-msvc.zip`. The archive contains exactly one file at the archive root: `sound-cabinet.exe`.
  - **SHA-256 step**: compute the digest over the archive (not the bare binary), write to `<archive-name>.sha256`. Unix uses `sha256sum` (Linux) or `shasum -a 256` (macOS); Windows uses PowerShell `Get-FileHash` and writes the lowercase-hex digest in the same `<digest>  <archive-name>\n` format.
- **MODIFIED**: `install.sh:download_and_verify()` and `install.sh:install_binary()`. Between verify and install, add a `tar -xzf "$archive" -C "$tmpdir"` step that produces a single `sound-cabinet` file at `$tmpdir/sound-cabinet`. The `install -m 755 …` step then operates on that extracted file. Dry-run mode prints the `[dry-run] tar -xzf …` line.
- **MODIFIED**: README `Quick install` and `Manual install` subsections to describe the archive form. The bash one-liner doesn't change — the script handles the extract internally. The Manual install section gains the Windows `.zip` mention.

### Install script: accept Intel Mac

- **MODIFIED**: `install.sh:detect_target_triple()`. The current implementation rejects `Darwin` + `x86_64` with an error. Change the case branch to map `Darwin/x86_64` → `x86_64-apple-darwin` and proceed normally.
- **MODIFIED**: existing `tests/install/darwin_x86_64_unsupported.test.sh` SHALL be replaced with `tests/install/darwin_x86_64_default_install.test.sh` exercising the happy path. (The error scenario was never reachable now that we ship Intel Mac binaries.)

### Test harness: tar shim + updated fixtures

- **NEW**: `tests/install/fixtures/tar` shim. Intercepts `tar -xzf <archive> -C <dir>` calls; if the archive name matches a fixture under `tests/install/fixtures/archives/`, writes a known-content `sound-cabinet` file to `<dir>` (e.g. `printf '#!/usr/bin/env bash\necho "fake"\n' > "$dir/sound-cabinet"; chmod 755 "$dir/sound-cabinet"`). Tests don't need real gzip-formatted fixtures this way.
- **MODIFIED**: `tests/install/fixtures/curl` shim. The URL-pattern dispatch SHALL match `*.tar.gz` (and `*.tar.gz.sha256`) where it previously matched bare-binary URLs. Add a HEAD-request branch (`-I` flag) so the script's `--version` validation path is testable: HEAD-200 if the archive fixture exists, exit 22 otherwise.
- **MODIFIED**: `tests/install/fixtures/archives/`. Replace bare-binary placeholder files with empty marker files at the new `.tar.gz`-suffixed names. Each marker has a sibling `.sha256` placeholder. The actual fixture content doesn't matter — the curl shim returns the marker bytes and the tar shim ignores the bytes and writes the synthetic binary.
- **NEW**: `tests/install/darwin_x86_64_default_install.test.sh` exercising the Intel Mac happy path. Replaces the deleted `darwin_x86_64_unsupported.test.sh`.
- **MODIFIED**: every existing `*.test.sh` that references archive URLs or filenames — update strings to use the `.tar.gz` suffix.

### What's NOT in this change

- **The `aarch64-linux` cross-toolchain setup, the `test` gate, the `publish` step, the `prerelease: contains(…, '-')` logic, the asset naming with version tag, the README structure**: all working correctly already; left untouched.
- **Code signing for macOS**: out of scope, same as the original installer spec.
- **Windows installer**: still no bash-installer support; Windows users use the new `.zip` archive via the Manual install path.

## Capabilities

### Modified Capabilities

- `release-pipeline`: 5 build matrix legs instead of 3; archive packaging instead of bare binaries.
- `install-script`: 4 supported triples for the bash installer (adds Intel Mac); download path includes a tar-extract step.

### New Capabilities

(None.)

## Impact

- **Affected specs**:
  - `release-pipeline` — MODIFIED requirements for the workflow trigger / matrix targets, MODIFIED requirements for the asset naming convention to reflect `.tar.gz`/`.zip`, MODIFIED requirement for "archives contain a single binary" extending to the Windows `.exe` case.
  - `install-script` — MODIFIED requirements for architecture detection (add Intel Mac), MODIFIED requirements for download/verify (add extract), MODIFIED harness requirement (tar shim, .tar.gz fixtures), MODIFIED README requirement (Manual install now documents Windows `.zip`).
- **Affected code**:
  - `install.sh` — bug fix at the empty-array site; new tar-extract step in `download_and_verify` (or a new sibling function); update `detect_target_triple` mapping for `Darwin/x86_64`.
  - `.github/workflows/release.yml` — restore 2 matrix legs, replace `cp` rename with `tar czf` (or `Compress-Archive` on Windows), update SHA-256 step to digest the archive, update artifact glob to match `.tar.gz`/`.zip`.
  - `tests/install/fixtures/curl` — match new URL suffixes; add HEAD-request branch.
  - `tests/install/fixtures/tar` — new shim.
  - `tests/install/fixtures/archives/` — rename existing marker files to use `.tar.gz` suffix; add Intel Mac fixtures.
  - `tests/install/darwin_x86_64_unsupported.test.sh` — DELETE and replace with `darwin_x86_64_default_install.test.sh`.
  - Other `*.test.sh` files — update archive-name strings to include the `.tar.gz` suffix.
  - `README.md` — Quick install and Manual install sections updated to mention archives + the Windows `.zip` path.
- **Operator-visible behavior after this change**:
  - 5 archives + 5 checksums per published release (currently 3 bare binaries + 3 checksums).
  - Intel Mac users get a working `curl … | bash` one-liner (currently it errors out with "out of scope; build from source").
  - Windows users see a `.zip` on the Releases page they can download manually (currently no Windows asset since the autocoder's workflow dropped that leg).
  - `bash tests/install/run.sh` exits 0 (currently 2 tests fail on real hosts).
- **Breaking**: no real-world consumers exist yet — no tags have been pushed, no one is depending on the bare-binary URLs. The README's curl one-liner stays the same (`install.sh` handles the extraction internally), so users who follow the README don't see any change.
- **Acceptance**:
  - `bash -n install.sh` clean.
  - `bash tests/install/run.sh` exits 0 with all 17+ test scenarios passing (the renamed Intel Mac test + the existing scenarios after their fixture-name updates).
  - `openspec validate expand-build-matrix-and-restore-archive-packaging --strict` passes.
  - The implementing agent CAN run all of the above in its sandbox without network access. The agent CANNOT verify the workflow runs end-to-end against a real tag — that's the maintainer's manual smoke-test step.

## Constraints visible to the implementing agent

- The agent SHALL NOT push tags or attempt to trigger the workflow. The release-pipeline acceptance signal is structural (YAML correctness, asset-naming interpolation, action pinning) — same constraint as the original `release-pipeline-github-actions` change.
- The agent SHALL NOT assume `shellcheck` or `actionlint` are available. Best-effort if present; print a single `WARN: <tool> not available in sandbox — skipping lint` line and proceed otherwise. Do NOT add either as a build prerequisite.
- The agent SHALL run `bash tests/install/run.sh` and verify it exits 0. The two currently-failing tests are the load-bearing acceptance signal for the bug-fix portion of this change.
- The agent SHALL NOT modify any baseline spec under `openspec/specs/` — the runtime behavior of `sound-cabinet` itself is unaffected by this change.
- The agent CANNOT verify the Windows `.zip` packaging works on a real Windows runner. Test the Windows path by structural inspection of the YAML (the `Compress-Archive` PowerShell command form, the SHA-256 computation via `Get-FileHash` with case-folding to lowercase, the asset-naming interpolation). Real Windows verification is the maintainer's manual step after a tag push.
- The agent CAN verify the macOS Intel path through the `uname` shim, same as the existing macOS aarch64 testing.
