## 1. Workflow file

- [ ] 1.1 Create `.github/workflows/release.yml`. Trigger: `on: push: tags: ['v*']`. Single workflow file with three jobs (`test`, `build`, `publish`) connected by `needs:` dependencies.
- [ ] 1.2 Top-level `permissions: contents: read` so the default is least-privilege. The `publish` job declares its own `permissions: contents: write` since `softprops/action-gh-release` needs it.
- [ ] 1.3 `test` job: `runs-on: ubuntu-22.04`. Steps:
  1. `actions/checkout@v4`
  2. `dtolnay/rust-toolchain@stable` (no override target; native ubuntu build chain)
  3. `sudo apt-get update && sudo apt-get install -y libasound2-dev` (needed by the `cpal` crate's build script)
  4. `Swatinem/rust-cache@v2` for incremental cargo cache
  5. `cargo test --release` (all targets, default feature set — matches the manual `cargo build --release` path documented in README).
- [ ] 1.4 `build` matrix job: `needs: test`, `strategy.fail-fast: false`, `strategy.matrix.include:` three entries. Each entry sets `target` and the `os` runner. Targets:
  - `target: x86_64-unknown-linux-gnu` / `os: ubuntu-22.04` / `cross: false`
  - `target: aarch64-unknown-linux-gnu` / `os: ubuntu-22.04` / `cross: true`
  - `target: aarch64-apple-darwin` / `os: macos-latest` / `cross: false`
- [ ] 1.5 `build` matrix steps (in order):
  1. `actions/checkout@v4`
  2. `dtolnay/rust-toolchain@stable` with `targets: ${{ matrix.target }}`
  3. **Linux x86_64 only**: `sudo apt-get update && sudo apt-get install -y libasound2-dev`. `if: matrix.target == 'x86_64-unknown-linux-gnu'`.
  4. **Linux aarch64 only**: `taiki-e/setup-cross-toolchain-action@v1` with `target: aarch64-unknown-linux-gnu`. This sets `CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER` and the right sysroot for ALSA cross-compile. `if: matrix.cross == true`.
  5. `Swatinem/rust-cache@v2` (keyed by matrix target)
  6. **Build**: `cargo build --release --target ${{ matrix.target }}`. For the cross case, `cross build --release --target ${{ matrix.target }}` is also acceptable if the setup-cross-toolchain action's sysroot doesn't cover ALSA cross-headers; resolve this at implementation time based on what actually works in CI.
  7. **Strip**: `strip target/${{ matrix.target }}/release/sound-cabinet` (skip on macOS where strip is implicit and the binary is much smaller anyway — gate this step with `if: runner.os != 'macOS'`).
  8. **Rename**: copy to a deterministic artifact name `sound-cabinet-${{ github.ref_name }}-${{ matrix.target }}` in the workspace root.
  9. **Checksum**: compute SHA-256. On Linux runners use `sha256sum`; on macOS use `shasum -a 256`. Output a file named `<binary-name>.sha256` with content exactly `<hex-digest>  <binary-name>\n` (two spaces between digest and filename — this is the format `sha256sum -c` and `shasum -a 256 -c` expect).
  10. `actions/upload-artifact@v4` with `name: binaries-${{ matrix.target }}` and the two files.
- [ ] 1.6 `publish` job: `needs: build`, `runs-on: ubuntu-22.04`, `permissions: contents: write`. Steps:
  1. `actions/download-artifact@v4` with `pattern: binaries-*` and `path: artifacts/` and `merge-multiple: true` (flattens all matrix artifacts into one directory).
  2. `softprops/action-gh-release@v2` with:
     - `tag_name: ${{ github.ref_name }}`
     - `files: artifacts/sound-cabinet-*`
     - `generate_release_notes: true`
     - `prerelease: ${{ contains(github.ref_name, '-') }}` — true for `v0.1.0-rc1`, `v0.1.0-dev.3`, etc.; false for `v0.1.0`, `v1.2.3`.
     - `fail_on_unmatched_files: true` so a missing artifact halts the publish instead of silently shipping a partial release.

## 2. Release procedure doc

- [ ] 2.1 Create `RELEASING.md` at the repo root. Target length ≤ 50 lines. Sections:
  - **Pre-flight**: `cargo test --release` is green on `master`; `Cargo.toml` `[package] version` is bumped to the new vX.Y.Z (no `v` prefix in Cargo.toml — that's just the tag).
  - **Cut the release**: `git tag vX.Y.Z && git push origin vX.Y.Z`. Workflow auto-publishes.
  - **Pre-release naming**: `vX.Y.Z-rc1`, `vX.Y.Z-dev.3`, `vX.Y.Z-beta.2`. Any tag containing `-` is flagged as a pre-release in the GitHub UI and excluded from `/releases/latest`. The install wizard surfaces only non-prerelease tags by default.
  - **After publish**: edit the release notes on GitHub if the auto-generated changelog needs annotation. Notify users via the appropriate channel.
  - **Verification**: cite the install script's `.sha256` verification step as the consumer of the checksum files.
  - **Yanking a release**: delete the GitHub Release and the tag (`git push --delete origin vX.Y.Z` and delete from the Releases UI). Note that anyone who already downloaded the binary still has it; yanking is best-effort.

## 3. README integration

- [ ] 3.1 In README, under the "## Install" section, replace the existing **"Pre-built binaries"** subsection's body. Keep the `curl -L … | tar xz` instructions for each platform but update the URLs to match the new asset naming. Pattern:
  - macOS aarch64: `curl -fL https://github.com/rbeverly/sound-cabinet/releases/latest/download/sound-cabinet-<TAG>-aarch64-apple-darwin -o sound-cabinet`. Replace `<TAG>` instructions with the actual download URL when the user picks a version.
  - The README change is minimal — the major install UX improvement comes from the companion `install-script-and-wizard` change, which will re-write this section with a curl one-liner.
  - Acceptance: README's pre-built-binaries instructions reference the *correct* asset names produced by this workflow, so a user who reads the README before the wizard ships can still install manually.

## 4. Spec delta

- [ ] 4.1 Author the ADDED requirement(s) in `specs/release-pipeline/spec.md` per the spec authored alongside this proposal. Validate with `openspec validate release-pipeline-github-actions --strict`.

## 5. Verification

- [ ] 5.1 `openspec validate release-pipeline-github-actions --strict` passes.
- [ ] 5.2 The implementing agent SHOULD attempt to lint the workflow with `actionlint` if it's available on PATH (`command -v actionlint && actionlint .github/workflows/release.yml`). If `actionlint` is not available in the sandbox, print one `WARN: actionlint not available in sandbox — skipping workflow lint` line and proceed. DO NOT add `actionlint` as a build prerequisite, install it as part of the change, or fail the implementation if it's absent.
- [ ] 5.3 Structural sanity checks the agent CAN do without GitHub:
  - YAML is valid (parse with `python3 -c "import yaml,sys; yaml.safe_load(open(sys.argv[1]))" .github/workflows/release.yml` or equivalent).
  - All `uses:` references are pinned to a major-version tag (`@v4`, `@v2`) or to a commit SHA. No `@main`, `@master`, or unpinned references.
  - The `permissions:` block is present at the top and `contents: write` is scoped to the publish job only.
  - The `prerelease:` expression uses `contains(github.ref_name, '-')` exactly as the proposal specifies.
  - The three matrix entries are present and reference the three target triples from the proposal.
  - Asset names interpolate `${{ github.ref_name }}` (the tag) and `${{ matrix.target }}` (the triple) and follow the `sound-cabinet-<tag>-<triple>` convention.
- [ ] 5.4 NOT in scope for this change's verification: actually pushing a smoke-test tag to verify the workflow runs end-to-end. That is the maintainer's manual acceptance step, documented in `RELEASING.md`. The implementing agent SHOULD NOT push tags.
