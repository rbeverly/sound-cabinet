# sheet-music-export Specification

## Purpose
Export a `.sc` score (or a slice of one) as LilyPond notation, or rendered as PDF via the LilyPond toolchain. Lets composers turn computed scores into readable sheet music for human players or printing.

## Requirements

### Requirement: `export` command writes LilyPond or PDF sheet music

The `export` subcommand SHALL accept a `.sc` source file and an `-o <output>` flag, and SHALL write either a LilyPond (`.ly`) source file or a rendered PDF, depending on the `--format` flag and the output extension.

The output format SHALL be determined as follows:
- If `--format pdf` is given, the format is PDF.
- If `--format ly` or `--format lilypond` is given, the format is LilyPond.
- If `--format` is not given but the output path ends in `.pdf`, the format is PDF.
- Otherwise the format defaults to LilyPond.

#### Scenario: Default to LilyPond when no format hint
- **WHEN** the user runs `sound-cabinet export <score.sc> -o out.ly`
- **THEN** a LilyPond source file is written to `out.ly`

#### Scenario: PDF inferred from output extension
- **WHEN** the user runs `sound-cabinet export <score.sc> -o out.pdf`
- **AND** `--format` is not specified
- **THEN** the output is rendered to PDF via the LilyPond toolchain

#### Scenario: Explicit PDF
- **WHEN** the user runs `sound-cabinet export <score.sc> -o out.ly --format pdf`
- **THEN** the output is rendered to PDF regardless of the extension

#### Scenario: Unknown format
- **WHEN** `--format` is given a value other than `lilypond`/`ly`/`pdf`
- **THEN** the program exits non-zero with `Unknown format: <value> (use 'lilypond' or 'pdf')`

#### Scenario: Missing `-o`
- **WHEN** the user runs `sound-cabinet export <score.sc>` without `-o`
- **THEN** the program exits non-zero with `-o <output> is required`

#### Scenario: Missing score path
- **WHEN** the user runs `sound-cabinet export -o out.ly` without a score path
- **THEN** the program exits non-zero with a usage message

### Requirement: PDF format requires the LilyPond toolchain

When the resolved format is PDF, the exporter SHALL invoke the LilyPond compiler (typically the external `lilypond` binary) to produce the PDF from the generated LilyPond source. If the LilyPond toolchain is not available, the exporter SHALL emit an error naming LilyPond as the missing dependency.

#### Scenario: LilyPond missing
- **WHEN** PDF export is requested AND the LilyPond binary is not found in `PATH`
- **THEN** the program exits non-zero with an error naming the missing LilyPond installation
- **AND** the intermediate `.ly` file may still be retained for the user

### Requirement: `--voice <name>` filters to a single voice's events

The `--voice <name>` flag SHALL filter the exported notation to only include events whose voice label matches `<name>`. Other voices SHALL be omitted from the output entirely.

#### Scenario: Export only the bass part
- **WHEN** the user runs `export <score.sc> -o bass.ly --voice bass`
- **THEN** the LilyPond output contains only the events labeled `bass`

### Requirement: `--source <pattern>` filters to one source pattern or section

The `--source <name>` flag SHALL filter the exported notation to only include events that originated from the named pattern, section, or top-level construct. Combined with `--voice`, both filters SHALL be applied (AND).

#### Scenario: Export a single section
- **WHEN** the user runs `export <score.sc> -o verse.ly --source verse_a`
- **THEN** only events whose source is `verse_a` are included

#### Scenario: Combined voice + source filter
- **WHEN** the user runs `export <score.sc> -o verse-bass.ly --source verse_a --voice bass`
- **THEN** only events labeled `bass` whose source is `verse_a` are included

### Requirement: `--from <beat>` and `--to <beat>` restrict the beat range

The `--from <beat>` and `--to <beat>` flags SHALL filter exported events to a beat range. `--from` is inclusive; `--to` is exclusive. Either or both MAY be supplied.

#### Scenario: Export a beat range
- **WHEN** the user runs `export <score.sc> -o slice.ly --from 0 --to 32`
- **THEN** only events with start beat ≥ 0 AND start beat < 32 are included

#### Scenario: Only `--from` specified
- **WHEN** only `--from 16` is given
- **THEN** all events with start beat ≥ 16 are included; no upper bound is applied

#### Scenario: Invalid `--from`/`--to` value
- **WHEN** `--from` or `--to` is given a non-numeric value
- **THEN** the program exits non-zero with `--from requires a number` or `--to requires a number`

### Requirement: `--key`, `--title`, and `--time` set notation metadata

The `--key <key>` flag SHALL set the LilyPond key signature (e.g. `Am`, `C`, `D`, `Bb`). The `--title <title>` flag SHALL set the score's title in the rendered output. The `--time <signature>` flag SHALL set the time signature (e.g. `4/4`, `3/4`). When `--time` is omitted, the default SHALL be `4/4`.

#### Scenario: Set key signature and title
- **WHEN** the user runs `export <score.sc> -o song.pdf --key Am --title "My Song"`
- **THEN** the rendered PDF shows title "My Song" and key signature A minor

#### Scenario: Default time signature
- **WHEN** `--time` is omitted
- **THEN** the notation uses 4/4 time

#### Scenario: Custom time signature
- **WHEN** `--time 3/4` is given
- **THEN** the notation uses 3/4 time
