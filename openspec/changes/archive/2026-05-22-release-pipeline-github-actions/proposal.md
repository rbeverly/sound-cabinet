## Why

Today sound-cabinet ships as `git clone + cargo build --release`. There's no `.github/workflows/release.yml`, no tagged releases on the GitHub Releases page, no pre-built binaries. Every new operator has to install the Rust toolchain, wait through a release build, and (on Linux) figure out the ALSA dev-package situation just to try the tool. That gates adoption at a step that has nothing to do with sound-cabinet itself.

The companion `install-script-and-wizard` change is a non-starter without per-tag pre-built binaries to download — this change is the substrate it depends on. But it's also useful on its own: a user who knows which target triple they want can pull the binary by hand from the GitHub Releases page without going through the curl-and-run installer.

The shape of "tag-triggered matrix-built binaries with checksums published to a GitHub Release" is well-trodden in the Rust ecosystem (`ripgrep`, `bat`, `fd`, `eza`). We follow the same conventions: gate on green tests, build a matrix of target triples, attach `.sha256` files, name assets predictably so a shell script can fetch them by URL pattern, and use the SemVer dash-suffix convention to auto-flag pre-releases.

## What Changes

- **NEW**: `.github/workflows/release.yml`, triggered on any tag matching `v*`. Three logical stages:
  1. **Test gate** (single job, runs first): `cargo test --release` on `ubuntu-22.04`. If this fails, no binaries are built and no release is published. The release is all-or-nothing.
  2. **Build matrix** (runs after the test gate passes): one job per target triple:
     - `x86_64-unknown-linux-gnu` — `ubuntu-22.04` runner; native build. Ubuntu 22.04 is chosen over `ubuntu-latest` so the binary's glibc baseline is widely compatible (works on Debian 11+, Ubuntu 22.04+, RHEL 9+, etc.). Installs `libasound2-dev` for the ALSA crate's build-time headers.
     - `aarch64-unknown-linux-gnu` — `ubuntu-22.04` runner with `cross` (`taiki-e/setup-cross-toolchain-action@v1`).
     - `aarch64-apple-darwin` — `macos-latest` runner (Apple Silicon, default since late 2024). Native cargo build.
     Each job: `cargo build --release --target <triple>`, `strip` the resulting binary, compute SHA-256, upload `sound-cabinet-<full-version-tag>-<triple>` and `sound-cabinet-<full-version-tag>-<triple>.sha256` as job artifacts.
  3. **Publish job** (runs after the build matrix completes): `runs-on: ubuntu-22.04`, downloads all artifacts via `actions/download-artifact@v4`, creates a GitHub Release for the tag using `softprops/action-gh-release@v2` with `generate_release_notes: true`, attaches all binaries + checksum files, and sets `prerelease: ${{ contains(github.ref_name, '-') }}` so any tag with a dash suffix (`v0.1.0-rc1`, `v0.1.0-dev.3`, `v0.1.0-alpha.1`) is auto-flagged as a pre-release. Stable tags (`v0.1.0`, `v1.2.3`) become full releases that GitHub's `/releases/latest` endpoint returns.
- **Asset naming convention** (pinned in this spec so the install script can rely on it across versions):
  - Binary: `sound-cabinet-<full-version-tag>-<rust-target-triple>` (no extension; the binary is just an ELF or Mach-O file).
  - Checksum: same name with `.sha256` appended, content in the format `<hex-digest>  <binary-name>\n` (two spaces between digest and name; matches what `sha256sum -c` and `shasum -a 256 -c` expect).
  - Examples:
    - `sound-cabinet-v0.1.0-x86_64-unknown-linux-gnu`
    - `sound-cabinet-v0.1.0-x86_64-unknown-linux-gnu.sha256`
    - `sound-cabinet-v0.2.0-rc1-aarch64-apple-darwin`
- **Permissions block** at the top of the workflow: `contents: read` by default; the `publish` job declares `contents: write` (required by `action-gh-release`). No other scopes.
- **NEW**: `RELEASING.md` at the repo root. Short (~30 lines) maintainer-facing doc explaining the release procedure: bump version in `Cargo.toml`, `git tag vX.Y.Z`, `git push --tags`, watch the workflow, edit release notes after publish. Includes the pre-release naming convention.
- **Windows** is explicitly out of scope for this change. The current README's Windows section will remain unchanged; the workflow does not build a Windows asset. Adding Windows can be a follow-up that adds a fourth matrix leg and a `.zip`-packaging step.

## Capabilities

### New Capabilities

- `release-pipeline`: a tag-triggered GitHub Actions workflow that produces matrix-built, checksum-attached binaries on the GitHub Releases page, with pre-release auto-detection from the SemVer dash-suffix convention.

### Modified Capabilities

(None — this change only adds a release workflow. No existing spec is modified.)

## Impact

- **Affected specs**: `release-pipeline` (new capability, one ADDED requirement establishing the workflow's contract). No existing specs are modified.
- **Affected code**:
  - `.github/workflows/release.yml` — new file.
  - `RELEASING.md` — new file at repo root.
  - No changes to `src/`, `Cargo.toml`, or the existing test suite.
- **Operator-visible behavior**: maintainers gain the ability to publish releases by pushing a tag. Sound-cabinet's GitHub Releases page becomes the canonical place to find pre-built binaries.
- **Cost**: GitHub Actions minutes per release ≈ one ubuntu test job (~3 min) + three build jobs (~8 min each in parallel) + one publish job (~1 min). Total ~15 min wall-clock per tag push. Well within the free tier for an open-source project.
- **Security**: binaries are built in GitHub-hosted runners from the tagged commit. Anyone can verify by inspecting the workflow log + downloading the release and re-building from source. SHA-256 sums attached to each binary let the install script (and operators) verify what they downloaded matches what the workflow produced. No artifact is signed beyond the sha256 — code signing for macOS / authenticode for Windows is out of scope.
- **Breaking**: no existing functionality changes. Source-build path remains supported and documented.
- **Acceptance**: pushing a tag like `v0.0.1-spec-smoke-test` (deletable post-verification) triggers the workflow, produces a release with three binaries + three checksum files, marked as pre-release because of the dash suffix. The workflow file passes `actionlint` without errors when run locally by the maintainer. The autocoder sandbox cannot verify the workflow actually runs on push — that requires GitHub. Treat the smoke-test tag push as the maintainer's manual acceptance step, documented in `RELEASING.md`.

## Constraints visible to the implementing agent

- The agent's sandbox cannot push to GitHub, cannot trigger Actions, and cannot verify a release was published. The acceptance signal is "the workflow file is correct YAML, passes any `actionlint` check that's available, references actions by pinned versions (or recognized aliases like `@v4`), and matches the contract described in `specs/release-pipeline/spec.md`."
- The agent cannot assume `actionlint` is installed in its sandbox. If `actionlint` is on PATH, run it and fail on errors. If not, the implementing agent SHOULD print a single `WARN: actionlint not available — workflow lint skipped in sandbox; will run in CI` line and proceed. CI (a separate concern outside this change's scope) can enforce `actionlint` strictly.
- The agent SHOULD NOT add `actionlint` as a new build prerequisite. It's a maintainer convenience, not part of the project's required toolchain.
- The agent cannot test the build matrix end-to-end. The acceptance criterion is structural correctness of the YAML, not "the workflow ran green." The first real validation is the maintainer pushing a smoke-test tag.
