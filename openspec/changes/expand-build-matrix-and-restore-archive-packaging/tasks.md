## 1. Fix the `maybe_sudo` empty-array bug

- [ ] 1.1 In `install.sh`, locate the `install_binary()` function (currently around line 191–202). The lines
  ```bash
  dry_or_run "${maybe_sudo[@]}" mkdir -p "$install_path"
  dry_or_run "${maybe_sudo[@]}" install -m 755 "$src" "$dst"
  ```
  fail under `set -u` when `maybe_sudo` is an empty array. Replace both expansions with the safe-empty idiom:
  ```bash
  dry_or_run ${maybe_sudo[@]+"${maybe_sudo[@]}"} mkdir -p "$install_path"
  dry_or_run ${maybe_sudo[@]+"${maybe_sudo[@]}"} install -m 755 "$src" "$dst"
  ```
  OR refactor to an explicit conditional that doesn't expand the empty array at all:
  ```bash
  if (( ${#maybe_sudo[@]} )); then
      dry_or_run "${maybe_sudo[@]}" mkdir -p "$install_path"
      dry_or_run "${maybe_sudo[@]}" install -m 755 "$src" "$dst"
  else
      dry_or_run mkdir -p "$install_path"
      dry_or_run install -m 755 "$src" "$dst"
  fi
  ```
  Either form is acceptable; pick whichever reads cleanest in context. Audit the rest of `install.sh` for any other `"${array[@]}"` site where the array can legitimately be empty under `set -u` and apply the same idiom.
- [ ] 1.2 Run `bash tests/install/run.sh` BEFORE making any other changes in this change. Confirm `dry_run_no_install.test.sh` and `user_flag_installs_to_home.test.sh` move from FAIL to PASS. If they don't, the bug fix is wrong — diagnose before proceeding to §2.

## 2. Workflow: add Intel Mac and Windows matrix legs

- [ ] 2.1 In `.github/workflows/release.yml`, locate the `build` job's `strategy.matrix.include` block (currently 3 entries). Add two more entries:
  ```yaml
  - target: x86_64-apple-darwin
    os: macos-latest
    cross: false
  - target: x86_64-pc-windows-msvc
    os: windows-latest
    cross: false
  ```
  Final ordering (recommended for readability — Linux first, then macOS, then Windows):
  - `x86_64-unknown-linux-gnu`
  - `aarch64-unknown-linux-gnu`
  - `aarch64-apple-darwin`
  - `x86_64-apple-darwin`
  - `x86_64-pc-windows-msvc`
- [ ] 2.2 Verify the existing conditional steps still gate correctly with the new entries:
  - `Install ALSA dev headers (Linux x86_64)` step's `if:` is `matrix.target == 'x86_64-unknown-linux-gnu'` — unchanged, correct.
  - `Set up cross toolchain (Linux aarch64)` step's `if:` is `matrix.cross == true` — unchanged, correct (only the aarch64-linux leg has `cross: true`).
  - `Strip binary (Linux)` step's `if:` is `runner.os != 'macOS'` — UPDATE to `runner.os == 'Linux'` so it skips both macOS AND Windows (Windows `strip.exe` isn't standard on the windows-latest runner).

## 3. Workflow: switch packaging from bare binary → archive

- [ ] 3.1 Replace the existing `Rename binary` step. For Unix runners (`runner.os != 'Windows'`):
  ```yaml
  - name: Package (Unix)
    if: runner.os != 'Windows'
    run: |
      tar czf "sound-cabinet-${{ github.ref_name }}-${{ matrix.target }}.tar.gz" \
        -C target/${{ matrix.target }}/release sound-cabinet
  ```
  This produces a `.tar.gz` whose sole entry is `sound-cabinet` at the archive root (mode 0755 — preserved by `tar c` from the build-output file). The `-C` flag avoids leading directory paths in the archive.
- [ ] 3.2 Add a Windows packaging step using PowerShell `Compress-Archive`:
  ```yaml
  - name: Package (Windows)
    if: runner.os == 'Windows'
    shell: pwsh
    run: |
      $archive = "sound-cabinet-${{ github.ref_name }}-${{ matrix.target }}.zip"
      Compress-Archive -Path "target/${{ matrix.target }}/release/sound-cabinet.exe" -DestinationPath $archive
  ```
  The `.zip` contains exactly one entry: `sound-cabinet.exe`.
- [ ] 3.3 Replace the existing SHA-256 steps. Three platform variants (the Windows one is new):
  - Linux:
    ```yaml
    - name: Compute SHA-256 (Linux)
      if: runner.os == 'Linux'
      run: |
        ARCHIVE="sound-cabinet-${{ github.ref_name }}-${{ matrix.target }}.tar.gz"
        sha256sum "$ARCHIVE" > "$ARCHIVE.sha256"
    ```
  - macOS:
    ```yaml
    - name: Compute SHA-256 (macOS)
      if: runner.os == 'macOS'
      run: |
        ARCHIVE="sound-cabinet-${{ github.ref_name }}-${{ matrix.target }}.tar.gz"
        DIGEST=$(shasum -a 256 "$ARCHIVE" | awk '{print $1}')
        printf '%s  %s\n' "$DIGEST" "$ARCHIVE" > "$ARCHIVE.sha256"
    ```
  - Windows:
    ```yaml
    - name: Compute SHA-256 (Windows)
      if: runner.os == 'Windows'
      shell: pwsh
      run: |
        $archive = "sound-cabinet-${{ github.ref_name }}-${{ matrix.target }}.zip"
        $digest = (Get-FileHash $archive -Algorithm SHA256).Hash.ToLower()
        "$digest  $archive`n" | Out-File -NoNewline -Encoding ASCII "$archive.sha256"
    ```
  The Windows variant uses `.zip`; the others use `.tar.gz`. All three produce a `.sha256` file in the format `<lowercase-hex>  <archive-filename>\n` (two spaces, single LF) that `sha256sum -c` and `shasum -a 256 -c` will accept verbatim.
- [ ] 3.4 Update the `Upload artifacts` step's `path:` to match both the new archive name AND the Windows `.zip`:
  ```yaml
  path: |
    sound-cabinet-${{ github.ref_name }}-${{ matrix.target }}.tar.gz
    sound-cabinet-${{ github.ref_name }}-${{ matrix.target }}.tar.gz.sha256
    sound-cabinet-${{ github.ref_name }}-${{ matrix.target }}.zip
    sound-cabinet-${{ github.ref_name }}-${{ matrix.target }}.zip.sha256
  ```
  Only the matching pair for the current matrix leg actually exists on the runner; `actions/upload-artifact` ignores missing paths by default. (If `actions/upload-artifact@v4` started failing on missing files in some future version, gate the `.tar.gz` lines on `runner.os != 'Windows'` and the `.zip` lines on `runner.os == 'Windows'`.)
- [ ] 3.5 The `publish` job's existing `files: artifacts/sound-cabinet-*` glob still matches everything correctly — the wildcard catches both `.tar.gz`, `.zip`, and their `.sha256` siblings. No change needed.

## 4. install.sh: accept Intel Mac

- [ ] 4.1 In `install.sh`, locate `detect_target_triple()` (currently around line 58–69). The line
  ```bash
  Darwin/x86_64) echo "ERROR: Intel Mac is out of scope for this installer; build from source per README." >&2; exit 1 ;;
  ```
  SHALL be replaced with
  ```bash
  Darwin/x86_64) TRIPLE="x86_64-apple-darwin" ;;
  ```
  Verify the surrounding catch-all branch (`*) echo "ERROR: no pre-built binary for ${OS}/${arch}; build from source per README." >&2; exit 1 ;;`) still catches genuinely unsupported combinations (armv7l, ppc64le, etc.) after the change.

## 5. install.sh: download archive, extract, install

- [ ] 5.1 In `download_and_verify()`, update the URL construction. The `base` variable currently equals the bare-binary name (`sound-cabinet-<tag>-<triple>`). Add the `.tar.gz` suffix to all references — the downloaded file is now an archive, not a binary. The function should download `<base>.tar.gz` and `<base>.tar.gz.sha256` and verify the archive's checksum.
  Rename the local variable to avoid confusion: introduce `local archive="${base}.tar.gz"` (or similar) and use `$archive` consistently for the downloaded file's filename. The `base` variable can stay as the un-suffixed prefix if reused later.
- [ ] 5.2 After successful verification but BEFORE the `install_binary` call, add an extract step. Either inline in `download_and_verify()` (immediately after the verify) or as a new sibling function `extract_archive(tag, tmpdir)`:
  ```bash
  extract_archive() {
      CURRENT_STEP="extract_archive"
      local tag="$1" tmpdir="$2"
      local archive="${BINARY_NAME}-${tag}-${TRIPLE}.tar.gz"
      dry_or_run tar -xzf "${tmpdir}/${archive}" -C "$tmpdir"
      if [[ $DRY_RUN -eq 0 ]]; then
          [[ -x "${tmpdir}/${BINARY_NAME}" ]] || { echo "ERROR: extracted archive did not contain ${BINARY_NAME}." >&2; exit 1; }
      fi
  }
  ```
  Call it from `main()` between `download_and_verify` and `install_binary`.
- [ ] 5.3 In `install_binary()`, update the `src` variable. Currently it points at the downloaded bare binary; now it should point at the extracted binary:
  ```bash
  local src="${tmpdir}/${BINARY_NAME}"
  ```
  (The extraction step ensured this file exists in the workspace.)
- [ ] 5.4 Verify `--dry-run` still does the right thing: the curl downloads are `[dry-run]`-only (already gated); the new `tar -xzf` is `[dry-run]`-only via `dry_or_run`; the install is `[dry-run]`-only (already gated). No real archive is downloaded, no extraction happens, no file lands on disk outside the harness tmpdir.

## 6. Test harness: tar shim + .tar.gz fixtures + Intel Mac happy path

- [ ] 6.1 Create `tests/install/fixtures/tar` (chmod +x). Behavior:
  ```bash
  #!/usr/bin/env bash
  # Test shim for tar. Recognizes `-xzf <archive> -C <dir>` invocations and
  # writes a known-content sound-cabinet binary to the target dir if a
  # corresponding fixture archive exists.
  set -euo pipefail
  archive=""
  dest=""
  while [[ $# -gt 0 ]]; do
      case "$1" in
          -xzf) archive="$2"; shift 2 ;;
          -C) dest="$2"; shift 2 ;;
          *) shift ;;
      esac
  done
  base=$(basename "$archive")
  if [[ -e "${TESTS_FIXTURES_DIR:-tests/install/fixtures}/archives/$base" ]]; then
      printf '#!/usr/bin/env bash\necho "fake sound-cabinet"\n' > "$dest/sound-cabinet"
      chmod 755 "$dest/sound-cabinet"
  else
      echo "tar shim: no fixture for $base" >&2
      exit 2
  fi
  ```
  The shim takes the same args the real `tar` does in our usage; tests don't need real gzip content.
- [ ] 6.2 Update `tests/install/fixtures/curl`. The current URL-pattern dispatch matches bare-binary URLs (`*/sound-cabinet-*-*` without `.sha256`). UPDATE the patterns:
  - `*/sound-cabinet-*-*.tar.gz` and `*/sound-cabinet-*-*.zip` → return matching archive fixture
  - `*/sound-cabinet-*-*.tar.gz.sha256` and `*/sound-cabinet-*-*.zip.sha256` → return matching .sha256 fixture
  - `*/repos/*/releases` → unchanged
  - Other URLs → exit 22
  Add HEAD-request handling: if curl is invoked with `-I` (or `--head`), emit a one-line `HTTP/1.1 200 OK\n` if the fixture exists, exit 22 if not. This is what `install.sh`'s `--version` validation path will need once it uses `curl -fsSI` to confirm a pinned tag exists.
- [ ] 6.3 Update `tests/install/fixtures/archives/`. The current directory has bare-binary placeholder files. Rename / replace them:
  - Delete any bare-binary files (`sound-cabinet-vX.Y.Z-<triple>` with no extension).
  - For each `(tag, triple)` combination tested, create an empty marker file `sound-cabinet-<tag>-<triple>.tar.gz` and a matching `<archive>.sha256` placeholder.
  - Add fixture archives for `x86_64-apple-darwin` (Intel Mac) for the relevant tags.
  - Keep the deliberately-wrong `.sha256` file used by `sha256_mismatch_aborts.test.sh` — but rename it to match the new `.tar.gz`-suffixed archive name.
- [ ] 6.4 Delete `tests/install/darwin_x86_64_unsupported.test.sh`. The Intel Mac scenario is now the happy path, not an error path.
- [ ] 6.5 Create `tests/install/darwin_x86_64_default_install.test.sh`. Structure:
  ```bash
  #!/usr/bin/env bash
  source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
  setup_test
  MOCK_UNAME_S=Darwin MOCK_UNAME_M=x86_64 \
      run_install --dry-run --non-interactive \
          --api-base-url http://test.local \
          --releases-base-url http://test.local/releases/download
  assert_equals 0 "$TEST_EXIT_CODE"
  assert_contains "$TEST_OUTPUT" "x86_64-apple-darwin"
  assert_contains "$TEST_OUTPUT" "[dry-run]"
  teardown_test
  ```
  (Match the pattern of the existing `darwin_aarch64_default_install.test.sh`.)
- [ ] 6.6 Update every existing `*.test.sh` that hard-codes archive URLs or filenames. Search for the old bare-binary form (`sound-cabinet-vX.Y.Z-<triple>` without an extension) and replace with the new `.tar.gz`-suffixed form. Specifically expect changes in:
  - `linux_x86_64_default_install.test.sh`
  - `linux_aarch64_default_install.test.sh`
  - `darwin_aarch64_default_install.test.sh`
  - `version_flag_pins_tag.test.sh`
  - `sha256_mismatch_aborts.test.sh`
  - `404_on_binary_url_aborts.test.sh` — also consider renaming the file to `404_on_archive_url_aborts.test.sh` for accuracy; the corresponding test in §7 spec scenarios uses the new name.
- [ ] 6.7 Verify `tests/install/lib.sh`'s `setup_test()` exports `TESTS_FIXTURES_DIR="$REPO_ROOT/tests/install/fixtures"` so the tar shim can find its `archives/` subdirectory regardless of cwd at invocation. If `setup_test` doesn't currently set this, add it.

## 7. README update

- [ ] 7.1 In `README.md`'s `## Install` section, locate the `### Quick install` subsection. Verify the curl one-liner is unchanged (the script handles extract internally — no user-visible URL change).
- [ ] 7.2 In the `### Manual install` subsection, update any URL or filename references to:
  - Unix targets: `sound-cabinet-<tag>-<triple>.tar.gz` + manual `curl … | tar xz`.
  - Windows: explicit mention of `sound-cabinet-<tag>-x86_64-pc-windows-msvc.zip` with PowerShell `Expand-Archive` instructions, OR a short note pointing at the GitHub Releases page where the user downloads via browser and extracts via their preferred tool.

## 8. Spec deltas

- [ ] 8.1 Author the MODIFIED requirements in `specs/release-pipeline/spec.md`. Each MODIFIED block contains the COMPLETE new requirement text (per OpenSpec convention — not a diff). The full text must match the previous requirement enough that openspec recognizes the rename, while reflecting the new 5-target / archive-packaged reality.
- [ ] 8.2 Author the MODIFIED requirements in `specs/install-script/spec.md` per the spec authored alongside this proposal.
- [ ] 8.3 Validate with `openspec validate expand-build-matrix-and-restore-archive-packaging --strict`.

## 9. Verification

- [ ] 9.1 `bash -n install.sh` passes (syntax check).
- [ ] 9.2 `bash tests/install/run.sh` passes — every `*.test.sh` exits 0, the runner reports zero failures. Both currently-failing tests (`dry_run_no_install.test.sh`, `user_flag_installs_to_home.test.sh`) move from FAIL to PASS, and the new `darwin_x86_64_default_install.test.sh` passes.
- [ ] 9.3 The implementing agent SHOULD attempt `shellcheck install.sh tests/install/*.sh tests/install/fixtures/*` if shellcheck is on PATH. If not, print one `WARN: shellcheck not available in sandbox — skipping lint` line and proceed. DO NOT install shellcheck.
- [ ] 9.4 The implementing agent SHOULD attempt `actionlint .github/workflows/release.yml` if actionlint is on PATH. Same WARN-and-proceed rule.
- [ ] 9.5 Structural sanity checks on `.github/workflows/release.yml`:
  - YAML parses cleanly.
  - `strategy.matrix.include` has exactly 5 entries with the triples listed in §2.1.
  - The Unix packaging step is gated on `runner.os != 'Windows'`; the Windows packaging step is gated on `runner.os == 'Windows'`.
  - All three SHA-256 steps emit `<lowercase-hex>  <archive-name>\n` to `<archive-name>.sha256`.
  - The `Strip binary` step's `if:` is `runner.os == 'Linux'` (was previously `runner.os != 'macOS'`).
  - The `publish` job's existing `files: artifacts/sound-cabinet-*` glob and `prerelease: ${{ contains(github.ref_name, '-') }}` are unchanged.
- [ ] 9.6 Structural sanity checks on `install.sh`:
  - The `Darwin/x86_64` branch in `detect_target_triple()` maps to `x86_64-apple-darwin`, NOT an error.
  - The download / verify / extract / install sequence calls `tar -xzf` between verify and install (gated by `dry_or_run` for dry-run).
  - No `"${array[@]}"` expansion on an array that could legitimately be empty (the bug-fix idiom is applied wherever needed).
  - `bash -n install.sh` clean.
- [ ] 9.7 `openspec validate expand-build-matrix-and-restore-archive-packaging --strict` passes.
- [ ] 9.8 NOT in scope for this change's verification: pushing a smoke-test tag, running the workflow against real GitHub, validating the Windows runner produces a working `.zip` from PowerShell, or testing the installer on a real Intel Mac. Those are maintainer-side smoke tests.
