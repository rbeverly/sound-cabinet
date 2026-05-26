## Why

Sound Cabinet existed as ~12K lines of working Rust code before any OpenSpec adoption. Establishing the project on a spec-driven workflow requires a baseline: an archived change that documents the existing capabilities so the archive is the authoritative source of truth and a future `rebuild-spec` from archive can reconstruct `openspec/specs/`.

Without this entry, the 11 baseline capability specs in `openspec/specs/` would be orphaned — present in the working tree but unreachable by any archive replay. That gap defeats the point of having the archive be authoritative.

This change is documentation-only. No Rust code, no workflow files, no test files are touched. The deliverable is the archive entry itself, dated before any subsequent functional change so that replaying the archive in date order reconstructs the project's spec state correctly.

## What Changes

- **NEW (archive only)**: 11 capability specs, each authored as `## ADDED Requirements` against an empty starting state. These reflect the implementation as it existed at the time the maintainer adopted OpenSpec for sound-cabinet.

The capability list, mapped to source modules:

| Capability | Covers | Source modules |
|---|---|---|
| `dsl-syntax` | Score language grammar, declarations, composition primitives, imports, lexical structure | `src/dsl/` |
| `audio-engine` | Synthesis primitives (oscillators, filters, envelopes, effects, panning, chord, arp, operators) | `src/engine/` |
| `master-bus` | HP/LP bookends, user chain, per-effect master directives, LUFS measurement and normalization | `src/engine/effects.rs` (MasterBus + stages) |
| `rendering` | `render` subcommand: WAV output, LUFS reporting, CLI overrides | `src/render/wav.rs`, `src/main.rs::cmd_render` |
| `playback` | `play` subcommand: realtime audio out, solo/vu/subfold/env/from flags, A/B master bypass | `src/render/realtime.rs`, `src/main.rs::cmd_play` |
| `watch-mode` | `watch` subcommand: file watcher + atomic engine swap on .sc save | `src/main.rs::cmd_watch` |
| `streaming-mode` | `stream` subcommand: stdin pipe-in, `at N` = N beats from now | `src/stream/`, `src/main.rs::cmd_stream` |
| `piano-mode` | `piano` subcommand: QWERTY + MIDI live play, velocity curves, sustain, recording | `src/main.rs::cmd_piano` |
| `algorithmic-generation` | `generate` subcommand: YAML pattern/drum/song files → .sc | `src/generate/`, `src/main.rs::cmd_generate` |
| `sheet-music-export` | `export` subcommand: LilyPond/PDF with voice/source/range/key/title/time filters | `src/export/`, `src/main.rs::cmd_export` |
| `mix-diagnostics` | `profile`, `test-master`, `freeze` subcommands | `src/main.rs::cmd_profile`, `cmd_test_master`, `cmd_freeze` |

## Capabilities

### New Capabilities

All 11 listed above. Each is established by this change.

### Modified Capabilities

(None — this is the project's baseline.)

## Impact

- **Affected specs**: 11 new capability specs added.
- **Affected code**: none. This is a documentation-only change that captures the project state as it was. The Rust code, the existing `.github/workflows/release.yml` (4-target), and the CLI surface all remain as they were.
- **Operator-visible behavior**: no change. The archive entry is internal SDD bookkeeping.
- **Breaking**: no.
- **Acceptance**:
  - Every `specs/<capability>/spec.md` file under this change has valid `## ADDED Requirements` format with at least one `### Requirement:` block.
  - Every requirement uses SHALL / SHALL NOT / MAY appropriately.
  - Every requirement has at least one `#### Scenario:` block.
  - `openspec validate 2026-05-21-initial-spec-baseline --strict` passes (note: the validate command treats the archived entry as if it were active).
  - Running the maintainer's `rebuild-spec` autocoder feature against the archive (initial-spec-baseline + 2026-05-22 entries + later modifications) reconstructs `openspec/specs/` modulo regenerable metadata (like the `## Purpose` paragraphs, which are not part of the delta convention).

## Constraints visible to the (hypothetical) implementing agent

This change is intentionally already archived rather than authored as an active change → implemented → archived. The reasons:

- The implementation already exists. There is nothing for an agent to build.
- The capability specs were authored directly into `openspec/specs/` as a one-time baseline capture, before OpenSpec's archive-as-source-of-truth convention was being honored. This backdated entry corrects that.
- Future capability changes SHALL follow the normal workflow: propose → review → implement → archive.

The archive entry is dated 2026-05-21 (one day before the 2026-05-22 changes) so that archive replay in date order applies the baseline before any subsequent modification.
