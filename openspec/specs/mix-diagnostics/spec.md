# mix-diagnostics Specification

## Purpose
TBD - created by archiving change initial-spec-baseline. Update Purpose after archive.
## Requirements
### Requirement: `profile` command reports per-voice levels and frequency-band energy

The `profile` subcommand SHALL accept a `.sc` source file, perform a full render in memory, and print two tables to stderr: a level table and a frequency-band breakdown.

The level table SHALL have columns `Voice`, `RMS`, `Peak`, `Relative`, `Crest`, `Status`. Rows SHALL be sorted by RMS descending. The `Relative` column SHALL be each voice's RMS minus the loudest voice's RMS. The `Status` column SHALL be one of:

- `INAUDIBLE` when RMS < -60 dB
- `Very quiet` when RMS < -40 dB
- `Quiet` when RMS < -24 dB
- `Dominant` when peak > -1 dB
- `Loudest` when relative > -3 dB
- `OK` otherwise

The frequency-band table SHALL have columns for `Sub` (below 80 Hz), `Low` (80–300 Hz), `Mid` (300 Hz – 3 kHz), `High` (above 3 kHz), and a per-voice `Status` warning column.

#### Scenario: Profile reports the level table
- **WHEN** the user runs `sound-cabinet profile <score.sc>`
- **THEN** stderr contains `Profiling <score.sc>...`
- **AND** stderr contains a level table with the six columns above, sorted by RMS descending
- **AND** each row's status reflects the rules above

#### Scenario: Profile reports the frequency-band table
- **WHEN** `profile` runs and the score contains voiced events
- **THEN** stderr also contains a per-voice frequency-band table with the four bands above
- **AND** voices with significant energy below 80 Hz are flagged `⚠ Sub-heavy`
- **AND** voices with little mid/high energy relative to low energy are flagged `⚠ No presence`
- **AND** voices that pass both checks are marked `OK`

#### Scenario: Empty score
- **WHEN** the score contains no voiced events
- **THEN** stderr contains `No voiced events found in the score.`
- **AND** no tables are printed

#### Scenario: Missing score path
- **WHEN** the user runs `sound-cabinet profile` with no arguments
- **THEN** the program exits non-zero with a usage message naming `<score.sc>`

### Requirement: `test-master` runs automated A/B comparison of the master bus

The `test-master` subcommand SHALL accept a `.sc` source file, render the score twice (once with the score's master bus chain applied, once with the user-definable portion of the master chain bypassed), and report the differences to stderr in terms of loudness (LUFS), crest factor, and frequency balance.

#### Scenario: A/B comparison report
- **WHEN** the user runs `sound-cabinet test-master <score.sc>`
- **THEN** the engine renders the score twice (master-active and master-bypassed)
- **AND** stderr contains a comparison of integrated loudness (LUFS) for both versions
- **AND** stderr contains a comparison of crest factor (peak − RMS) for both versions
- **AND** stderr contains a per-band frequency balance comparison

#### Scenario: Always-present bookends still apply when bypassed
- **WHEN** `test-master` produces the "bypassed" version
- **THEN** only the user-definable portion of the master chain is removed
- **AND** the always-present HP 30 Hz, LP 18 kHz, and brick-wall limiter bookends remain active in the bypassed render

### Requirement: `freeze` command expands all randomness into a flat `.sc`

The `freeze` subcommand SHALL accept a `.sc` source file, fully resolve all patterns, sections, `pick`/`shuffle` randomization, `repeat` blocks, swing, and humanize into an explicit list of absolute `at <beat> play ... for ... beats` events, and emit the result as valid `.sc` source — either to stdout or, with `-o <path>`, to a file.

#### Scenario: Print frozen score to stdout
- **WHEN** the user runs `sound-cabinet freeze <score.sc>`
- **THEN** stdout contains valid `.sc` source
- **AND** the source begins with the comment `// Frozen from <score.sc>`
- **AND** all patterns/sections/repeat/pick have been replaced by absolute `at` events
- **AND** voice/wave/master/normalize/bpm/pedal directives from the original score are preserved verbatim

#### Scenario: Write frozen score to a file
- **WHEN** the user runs `sound-cabinet freeze <score.sc> -o frozen.sc`
- **THEN** the contents are written to `frozen.sc` instead of stdout
- **AND** stderr contains `Frozen to frozen.sc`

#### Scenario: Deterministic seeding
- **WHEN** the user runs `sound-cabinet freeze <score.sc> --seed 42`
- **THEN** all `pick`/`shuffle` and humanize randomness is generated from a RNG seeded with 42
- **AND** the frozen output begins with `// Seed: 42`
- **AND** running with the same seed twice produces byte-identical output (apart from runtime-irrelevant whitespace)

#### Scenario: Frozen output is audio-equivalent to source
- **WHEN** the user runs `sound-cabinet play frozen.sc` after freezing
- **THEN** the audible result matches the playback of the original score for the same seed
- **AND** patterns are not re-expanded (the events are already absolute)

#### Scenario: Frozen events include source comments
- **WHEN** an event was scheduled by a pattern or with a voice label
- **THEN** the frozen `at` line includes a trailing comment of the form `// <source_pattern>, voice:<voice_label>` (omitting either component when not applicable, and omitting the `voice:` part when the voice label equals the source name)

#### Scenario: Missing score path
- **WHEN** the user runs `sound-cabinet freeze` with no arguments
- **THEN** the program exits non-zero with a usage message naming `<score.sc>`

### Requirement: `render` emits a per-voice level summary automatically

After every successful `render` invocation, the engine SHALL print the same per-voice level summary table that `profile` produces (the level table portion, without the frequency-band table). This gives equivalent diagnostic information without requiring a separate `profile` run.

#### Scenario: Render auto-summary
- **WHEN** the user runs `sound-cabinet render <score.sc> -o out.wav` and the render succeeds
- **THEN** stderr contains a per-voice level summary table with `Voice`, `RMS`, `Peak`, `Status` columns
- **AND** the table is sorted by RMS descending

