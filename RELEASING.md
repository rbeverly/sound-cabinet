# Releasing sound-cabinet

Tagged pushes (`v*`) trigger `.github/workflows/release.yml`, which runs tests,
builds matrix binaries (`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`,
`aarch64-apple-darwin`), and publishes a GitHub Release with binaries + `.sha256`
files. Windows is out of scope.

## Pre-flight

1. `cargo test --release` is green on `master`.
2. Bump `[package] version` in `Cargo.toml` to the new `X.Y.Z` (no `v` prefix —
   that's only on the tag). Commit and push to `master`.

## Cut the release

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

The workflow auto-publishes once all three matrix legs succeed. Watch it at
`https://github.com/rbeverly/sound-cabinet/actions`.

## Pre-release naming

Any tag containing `-` is flagged as a pre-release by the workflow's
`prerelease: ${{ contains(github.ref_name, '-') }}` expression:
`vX.Y.Z-rc1`, `vX.Y.Z-dev.3`, `vX.Y.Z-beta.2`, `vX.Y.Z-alpha.1`. Pre-releases
are excluded from `/releases/latest`, so the install wizard's "latest stable"
default skips them.

## After publish

Edit the auto-generated release notes on GitHub if the changelog needs
annotation, then notify users via the appropriate channel.

## Verification

Each binary has a matching `<binary>.sha256` file with content
`<hex-digest>  <binary-name>\n` (two spaces) — accepted by both `sha256sum -c`
(Linux) and `shasum -a 256 -c` (macOS). The install script (`install.sh`)
downloads both files and runs the verifier before installing.

## Yanking a release

```bash
git push --delete origin vX.Y.Z
```

Then delete the GitHub Release from the Releases UI. Anyone who already
downloaded the binary still has a copy; yanking is best-effort.
