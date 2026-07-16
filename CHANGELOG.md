# Changelog

All notable changes to Sound Cabinet are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [v0.12.2] - 2026-05-26

### Fixed
- Fix the `aarch64-unknown-linux-gnu` release build by cross-compiling with `cross` + `Cross.toml`, and strip every target uniformly via a Cargo profile.

## [v0.12.1] - 2026-05-26

### Security
- Bound the arpeggiator step count so a huge `arp` rate literal (for example `arp(A4, 1e18)`) can no longer request a giant allocation and abort the renderer.
- Detect self-referencing and cyclic section/pattern references while expanding a score, instead of recursing until the thread stack overflows.

### Also included
- Adopted the OpenSpec spec-driven workflow and captured the existing engine, DSL, and CLI as the archive baseline (no runtime change).

## [v0.12.0] - 2026-05-26

### Added
- Restore the full five-target release build matrix — Linux x86_64, Linux aarch64, macOS Apple Silicon, macOS Intel, and Windows — publishing each as a SHA-256-checksummed `.tar.gz` (or `.zip` on Windows) archive instead of a bare binary.
- Support installing on Intel Macs via `install.sh`.

### Fixed
- Fix an `install.sh` crash under bash strict mode (`set -u`) when installing with `--user` or `--prefix`.

## [v0.11.1] - 2026-04-04

### Added
- Add master fade-in / fade-out and per-pattern gain envelopes.

## [v0.11.0] - 2026-04-04

### Changed
- Overhaul the master bus (major upgrade).

### Fixed
- Fix a multiband compressor bug.

## [v0.10.0] - 2026-04-02

### Added
- Expand mastering with soft clipping, a master EQ curve, frequency-band profiling, a multiband compressor, upward compression, and a harmonic exciter.
- Add environment simulation to audition a mix on different playback systems (highway, subway, phone speaker, and more), plus sub-bass fold-up monitoring.

### Changed
- Major toolchain upgrade with more sensible defaults out of the box.

## [v0.9.0] - 2026-04-02

### Added
- Add stereo output.

### Fixed
- Stereo output fixes and a LUFS sanity check.

## [v0.8.1] - 2026-04-01

### Added
- Add `--solo` to isolate a part by tag name or instrument.

## [v0.8.0] - 2026-04-01

### Changed
- Redesign chord notation.

### Fixed
- Assorted chord-notation bug fixes.

## [v0.7.0] - 2026-03-31

### Changed
- Sections are now much more flexible.

### Fixed
- Fix a section-summary truncation bug.

## [v0.6.1] - 2026-03-31

### Changed
- VU meter improvements.

## [v0.6.0] - 2026-03-31

### Added
- Add VU meters, per-instrument sound normalization, and output-level tools.

### Changed
- Improve instrument/voice labeling and sustain-pedal behavior.

## [v0.5.0] - 2026-03-24

### Added
- Add sheet-music export to LilyPond and PDF.
- Add a basic melody and harmony generator, and percussion to the generator.
- Add `--from` to start playback at a given beat.
- Add master gain.

### Changed
- Overhaul the arpeggiator: it now plays real instruments reliably, with direction, octave spanning, gate, and speed controls.
- Support MIDI instruments in piano mode, with live note duration and sustain so piano-mode playback and recordings match.

## [v0.4.0] - 2026-03-20

### Added
- Add master-compressor attack/release and sidechain controls.
- Add Windows installation and prebuilt binary releases.
- Add `-v` / `--verbose` debug output for section playback.

## [v0.3.1] - 2026-03-20

### Added
- Add a master EQ.

## [v0.3.0] - 2026-03-20

### Added
- Add a master bus with LUFS measurement and target-loudness normalization, plus a `loudness(freq)` Fletcher-Munson compensation function.
- Add the `with` keyword for instrument substitution and placeholders in patterns and loops.

## [v0.2.1] - 2026-03-20

### Added
- Add a bandpass filter, a gate effect, and mid-score BPM changes.

### Fixed
- Bug fixes and architectural refactoring; rebalance the `degrade` effect.

## [v0.2.0] - 2026-03-20

### Added
- Add an instrument system with a bundled instrument library, custom waveforms, and a pulse oscillator with PWM sweeps.
- Add low-pass and high-pass filter sweeps, compression, and lo-fi processing (bit-crush, decimate, degrade, and wet/dry mix).
- Add chord notation, a sustain pedal, and swing / humanize jitter at the document or section level.
- Add a `watch` mode that re-renders on save, and improve noise support.

## [v0.1.2] - 2026-03-18

### Added
- Add initial effects scaffolding and note notation.
- Add swell and improve envelopes.

### Changed
- Rework the arpeggiator.

## [v0.1.1] - 2026-03-17

### Added
- Initial release: render `.sc` compositions to audio, with composition-of-compositions and a `decay()` envelope.

[Unreleased]: https://github.com/rbeverly/sound-cabinet/compare/v0.12.2...HEAD
[v0.12.2]: https://github.com/rbeverly/sound-cabinet/compare/v0.12.1...v0.12.2
[v0.12.1]: https://github.com/rbeverly/sound-cabinet/compare/v0.12.0...v0.12.1
[v0.12.0]: https://github.com/rbeverly/sound-cabinet/compare/v0.11.1...v0.12.0
[v0.11.1]: https://github.com/rbeverly/sound-cabinet/compare/v0.11.0...v0.11.1
[v0.11.0]: https://github.com/rbeverly/sound-cabinet/compare/v0.10.0...v0.11.0
[v0.10.0]: https://github.com/rbeverly/sound-cabinet/compare/v0.9.0...v0.10.0
[v0.9.0]: https://github.com/rbeverly/sound-cabinet/compare/v0.8.1...v0.9.0
[v0.8.1]: https://github.com/rbeverly/sound-cabinet/compare/v0.8.0...v0.8.1
[v0.8.0]: https://github.com/rbeverly/sound-cabinet/compare/v0.7.0...v0.8.0
[v0.7.0]: https://github.com/rbeverly/sound-cabinet/compare/v0.6.1...v0.7.0
[v0.6.1]: https://github.com/rbeverly/sound-cabinet/compare/v0.6.0...v0.6.1
[v0.6.0]: https://github.com/rbeverly/sound-cabinet/compare/v0.5.0...v0.6.0
[v0.5.0]: https://github.com/rbeverly/sound-cabinet/compare/v0.4.0...v0.5.0
[v0.4.0]: https://github.com/rbeverly/sound-cabinet/compare/v0.3.1...v0.4.0
[v0.3.1]: https://github.com/rbeverly/sound-cabinet/compare/v0.3.0...v0.3.1
[v0.3.0]: https://github.com/rbeverly/sound-cabinet/compare/v0.2.1...v0.3.0
[v0.2.1]: https://github.com/rbeverly/sound-cabinet/compare/v0.2.0...v0.2.1
[v0.2.0]: https://github.com/rbeverly/sound-cabinet/compare/v0.1.2...v0.2.0
[v0.1.2]: https://github.com/rbeverly/sound-cabinet/compare/v0.1.1...v0.1.2
[v0.1.1]: https://github.com/rbeverly/sound-cabinet/releases/tag/v0.1.1
