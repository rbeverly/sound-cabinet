# song-generation Specification

## ADDED Requirements

### Requirement: Pattern files must be internally consistent

`PatternFile::validate` SHALL reject a pattern file that is structurally
inconsistent, returning an error that identifies the offending pattern before it
is used for generation. A pattern file SHALL provide either an explicit
`rhythm` + `contour` pair or a `motif`; when a direct pattern is given, its
`contour` length SHALL equal its `rhythm` length, any non-empty `emphasis` array
SHALL match the `rhythm` length, and each rest position SHALL be a rest in both
`rhythm` and `contour`; when a `motif` is given, its `contour` length SHALL
equal its `rhythm` length and any non-empty motif `emphasis` array SHALL match
the motif `rhythm` length.

#### Scenario: Pattern with neither rhythm+contour nor motif is rejected

- **GIVEN** a pattern file with `name`, `type`, and `time` but no `rhythm`,
  `contour`, or `motif`
- **WHEN** the pattern is loaded
- **THEN** validation returns an error indicating it must have either
  rhythm+contour or motif

#### Scenario: Direct emphasis length mismatch is rejected

- **GIVEN** a pattern whose `rhythm.hits` has 2 entries, `contour` has 2
  entries, and `emphasis` has 1 entry
- **WHEN** the pattern is loaded
- **THEN** validation returns an error reporting the emphasis/rhythm length
  mismatch

#### Scenario: Contour rest not aligned to a rhythm rest is rejected

- **GIVEN** a pattern whose `rhythm.hits` is `["1/4", "1/4"]` and whose
  `contour` is `[root, "~"]`
- **WHEN** the pattern is loaded
- **THEN** validation returns an error identifying the misaligned contour rest
  position

#### Scenario: Motif rhythm/contour length mismatch is rejected

- **GIVEN** a pattern whose `motif.rhythm` has 2 entries and `motif.contour` has
  1 entry
- **WHEN** the pattern is loaded
- **THEN** validation returns an error reporting the motif rhythm/contour length
  mismatch

#### Scenario: Motif emphasis length mismatch is rejected

- **GIVEN** a pattern whose `motif.rhythm` has 2 entries, `motif.contour` has 2
  entries, and `motif.emphasis` has 1 entry
- **WHEN** the pattern is loaded
- **THEN** validation returns an error reporting the motif emphasis/rhythm
  length mismatch

### Requirement: Song files must define parts and a valid arrangement

`SongFile::validate` SHALL reject a song file that defines no parts, whose
arrangement is empty, or whose per-part motif `contour` length differs from its
`rhythm` length, returning an error that identifies the offending song or part
before generation.

#### Scenario: Song with no parts is rejected

- **GIVEN** a song file whose `parts` map is empty and whose `arrangement` is
  `[verse]`
- **WHEN** the song is loaded
- **THEN** validation returns an error indicating it must have at least one part

#### Scenario: Song with an empty arrangement is rejected

- **GIVEN** a song file with one valid part and an empty `arrangement`
- **WHEN** the song is loaded
- **THEN** validation returns an error indicating the arrangement must have at
  least one entry

#### Scenario: Per-part motif rhythm/contour mismatch is rejected

- **GIVEN** a song whose part `verse` has a motif with `rhythm: ["1/4", "1/4"]`
  and `contour: [root]`, arranged as `[verse]`
- **WHEN** the song is loaded
- **THEN** validation returns an error identifying the part `verse` and the
  motif rhythm/contour length mismatch

### Requirement: Drum patterns must define voices with consistent emphasis

`DrumPattern::validate` SHALL reject a drum pattern that defines no voices, or
any voice whose non-empty `emphasis` array length differs from its `rhythm`
length, returning an error that identifies the offending pattern or voice.

#### Scenario: Drum pattern with no voices is rejected

- **GIVEN** a drum pattern whose `voices` array is empty
- **WHEN** the drum pattern is loaded
- **THEN** validation returns an error indicating it must have at least one
  voice

#### Scenario: Drum voice emphasis length mismatch is rejected

- **GIVEN** a drum pattern with a voice whose `rhythm` has 2 entries and
  `emphasis` has 1 entry
- **WHEN** the drum pattern is loaded
- **THEN** validation returns an error reporting the voice's emphasis/rhythm
  length mismatch
