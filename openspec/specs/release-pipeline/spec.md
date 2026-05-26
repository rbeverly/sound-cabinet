# release-pipeline Specification

## Purpose
Define the GitHub Actions workflow that builds release binaries from tag pushes, attaches per-binary `.sha256` checksums, publishes to GitHub Releases with automatic pre-release detection from the SemVer dash-suffix convention, and the maintainer-facing release procedure documentation.
## Requirements
### Requirement: Tag-triggered release workflow produces architecture-specific binaries with checksums on GitHub Releases

The repository SHALL include a GitHub Actions workflow at `.github/workflows/release.yml` that is triggered on push of any tag matching `v*`. The workflow SHALL execute in three stages: a test gate, a matrix build, and a publish step. A test failure SHALL abort the entire workflow before any binaries are built; binaries SHALL only be uploaded as artifacts when the test job is green. The publish step SHALL produce a single GitHub Release for the tag containing every matrix archive and its accompanying `.sha256` file, and SHALL mark the release as a pre-release when the tag name contains a hyphen.

The workflow SHALL build for these target triples:

- `x86_64-unknown-linux-gnu` (native build on an Ubuntu 22.04 runner)
- `aarch64-unknown-linux-gnu` (cross-compiled on an Ubuntu 22.04 runner)
- `aarch64-apple-darwin` (native build on `macos-latest`)
- `x86_64-apple-darwin` (native build on `macos-latest`)
- `x86_64-pc-windows-msvc` (native build on `windows-latest`)

Each Unix target SHALL be packaged as a `.tar.gz` containing exactly one file at the archive root: the `sound-cabinet` binary, with mode 0755. The Windows target SHALL be packaged as a `.zip` containing exactly one file at the archive root: `sound-cabinet.exe`. Linux binaries SHALL be stripped of debug symbols before packaging; macOS strip is implicit in release builds and SHALL NOT be invoked separately; Windows binaries are not stripped.

#### Scenario: Stable tag publishes a full release with five archives and five checksums
- **WHEN** a tag matching `^v[0-9]+\.[0-9]+\.[0-9]+$` (e.g. `v0.1.0`, `v1.2.3`) is pushed
- **THEN** the test gate runs first and succeeds
- **AND** five matrix build jobs run in parallel after the test job, one per target triple
- **AND** each Unix build job produces a `.tar.gz` archive named `sound-cabinet-<tag>-<triple>.tar.gz` and a checksum file `sound-cabinet-<tag>-<triple>.tar.gz.sha256`
- **AND** the Windows build job produces a `.zip` archive named `sound-cabinet-<tag>-x86_64-pc-windows-msvc.zip` and a checksum file `sound-cabinet-<tag>-x86_64-pc-windows-msvc.zip.sha256`
- **AND** the publish job downloads all ten artifacts and creates a GitHub Release for the tag containing all ten files
- **AND** the GitHub Release is NOT marked as a pre-release
- **AND** the GitHub Release appears as `/releases/latest` for the repository

#### Scenario: Pre-release tag is marked as prerelease
- **WHEN** a tag containing a hyphen (e.g. `v0.1.0-rc1`, `v0.1.0-dev.3`, `v0.2.0-alpha.1`) is pushed
- **THEN** the workflow runs the same test-then-build-then-publish stages
- **AND** the resulting GitHub Release is marked with `prerelease: true`
- **AND** the GitHub Release does NOT replace `/releases/latest`

#### Scenario: Test failure aborts the release
- **WHEN** the `test` job fails on a tag push
- **THEN** no build job runs
- **AND** no GitHub Release is created for the tag

#### Scenario: One matrix leg failure halts the publish
- **WHEN** all five build matrix legs run AND any one of them fails
- **THEN** the publish job does NOT run (the `needs: build` gate fails)
- **AND** no GitHub Release is created for the tag

#### Scenario: Linux archive contains a stripped binary at the root
- **WHEN** an auditor extracts `sound-cabinet-v0.1.0-x86_64-unknown-linux-gnu.tar.gz`
- **THEN** the archive yields a single file named `sound-cabinet` with mode 0755
- **AND** running `file sound-cabinet` reports a stripped ELF executable

#### Scenario: macOS archive contains the binary at the root
- **WHEN** an auditor extracts `sound-cabinet-v0.1.0-aarch64-apple-darwin.tar.gz` (or `-x86_64-apple-darwin.tar.gz`)
- **THEN** the archive yields a single file named `sound-cabinet` with mode 0755
- **AND** running `file sound-cabinet` reports a Mach-O executable for the matching architecture

#### Scenario: Windows archive contains sound-cabinet.exe at the root
- **WHEN** an auditor extracts `sound-cabinet-v0.1.0-x86_64-pc-windows-msvc.zip`
- **THEN** the archive yields a single file named `sound-cabinet.exe`

### Requirement: Asset naming convention is pinned

The workflow SHALL name release assets according to a fixed convention so consumers (the install script, manual downloaders, automated mirrors) can construct download URLs deterministically given a tag and target triple. The convention SHALL include the full tag in every asset filename so a downloaded file is unambiguous outside the GitHub Release context.

The archive asset name SHALL be exactly `sound-cabinet-<full-version-tag>-<rust-target-triple><suffix>` where:

- `<full-version-tag>` is the literal tag name including its leading `v` and any dash-suffix (e.g. `v0.1.0`, `v0.2.0-rc1`)
- `<rust-target-triple>` is one of the five supported triples
- `<suffix>` is `.tar.gz` for Unix targets (Linux x86_64, Linux aarch64, macOS aarch64, macOS x86_64), `.zip` for the Windows target

The checksum file SHALL be named exactly `<archive-name>.sha256` (i.e. append `.sha256` to the full archive filename including its extension). Its content SHALL be the SHA-256 digest in lowercase hex followed by two ASCII spaces followed by the archive's filename followed by a newline — the format consumed by both `sha256sum -c` (Linux) and `shasum -a 256 -c` (macOS) without modification. The Windows runner SHALL compute the digest via PowerShell `Get-FileHash` and SHALL force the output to lowercase hex so the resulting `.sha256` file is byte-identical in format to its Unix siblings.

#### Scenario: Asset names match the convention
- **WHEN** a tag `v0.1.0` is pushed and the workflow completes
- **THEN** the GitHub Release contains exactly these ten files:
  - `sound-cabinet-v0.1.0-x86_64-unknown-linux-gnu.tar.gz`
  - `sound-cabinet-v0.1.0-x86_64-unknown-linux-gnu.tar.gz.sha256`
  - `sound-cabinet-v0.1.0-aarch64-unknown-linux-gnu.tar.gz`
  - `sound-cabinet-v0.1.0-aarch64-unknown-linux-gnu.tar.gz.sha256`
  - `sound-cabinet-v0.1.0-aarch64-apple-darwin.tar.gz`
  - `sound-cabinet-v0.1.0-aarch64-apple-darwin.tar.gz.sha256`
  - `sound-cabinet-v0.1.0-x86_64-apple-darwin.tar.gz`
  - `sound-cabinet-v0.1.0-x86_64-apple-darwin.tar.gz.sha256`
  - `sound-cabinet-v0.1.0-x86_64-pc-windows-msvc.zip`
  - `sound-cabinet-v0.1.0-x86_64-pc-windows-msvc.zip.sha256`

#### Scenario: Checksum file format is compatible with sha256sum -c
- **WHEN** the workflow produces `sound-cabinet-v0.1.0-x86_64-unknown-linux-gnu.tar.gz.sha256`
- **THEN** the file contains exactly one line of the form `<64-hex-chars>  sound-cabinet-v0.1.0-x86_64-unknown-linux-gnu.tar.gz\n`
- **AND** running `cd <dir-containing-both-files> && sha256sum -c sound-cabinet-v0.1.0-x86_64-unknown-linux-gnu.tar.gz.sha256` succeeds with output containing `OK`

#### Scenario: Windows checksum uses lowercase hex
- **WHEN** the Windows build job computes the SHA-256 via PowerShell `Get-FileHash`
- **THEN** the resulting `.sha256` file contains the digest in lowercase hex (NOT the uppercase that `Get-FileHash` returns by default)
- **AND** the file is otherwise byte-identical in format to its Linux/macOS siblings (two spaces between digest and filename, single trailing newline)

#### Scenario: Pre-release tag preserves the full tag in the asset name
- **WHEN** the tag `v0.2.0-rc1` is pushed
- **THEN** assets are named with `v0.2.0-rc1` verbatim (e.g. `sound-cabinet-v0.2.0-rc1-aarch64-apple-darwin.tar.gz`)
- **AND** the dash-suffix is NOT stripped from the asset name

### Requirement: Workflow follows least-privilege permission model

The workflow file SHALL declare a top-level `permissions: contents: read` block so that, by default, all jobs run with read-only access to the repository. The `publish` job SHALL declare its own job-level `permissions: contents: write` so that the GitHub Release can be created. No other write scopes (`packages: write`, `id-token: write`, etc.) SHALL be requested.

#### Scenario: Top-level permissions are read-only
- **WHEN** an auditor inspects `.github/workflows/release.yml`
- **THEN** the file contains a top-level `permissions:` block with `contents: read` and no other entries

#### Scenario: Publish job is the only write-scoped job
- **WHEN** an auditor inspects the `publish` job
- **THEN** the job declares `permissions: contents: write`
- **AND** the `test` and `build` jobs declare no job-level `permissions` block (inheriting the read-only default)

### Requirement: Action references are pinned

Every `uses:` entry in the workflow SHALL reference a published action and SHALL pin to a major-version tag (`@v4`) or a commit SHA. The workflow SHALL NOT reference `@main`, `@master`, or any unpinned ref. This protects against supply-chain compromise of a referenced action.

#### Scenario: All action references pinned
- **WHEN** an auditor inspects every `uses:` reference in the workflow
- **THEN** each reference matches either `@v[0-9]+` or `@[0-9a-f]{40}`
- **AND** none reference `@main`, `@master`, `@latest`, or a bare branch name

### Requirement: RELEASING.md documents the maintainer release procedure

The repository SHALL include a `RELEASING.md` file at the repo root that documents the maintainer's release procedure. The doc SHALL cover: pre-flight (tests green, version bump), the tag-and-push command sequence, the pre-release naming convention, the post-publish step of editing release notes, and how the `.sha256` files are consumed by the install script.

#### Scenario: RELEASING.md exists and covers the required topics
- **WHEN** an auditor reads `RELEASING.md`
- **THEN** the doc explains how to cut a release in ≤ 50 lines
- **AND** the doc documents the SemVer dash-suffix convention as the way to publish a pre-release
- **AND** the doc references `Cargo.toml`'s `[package] version` as the place to bump before tagging

