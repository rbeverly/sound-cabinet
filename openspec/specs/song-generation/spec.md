# song-generation Specification

## Purpose
TBD - created by archiving change fix-song-empty-part-chords-panic. Update Purpose after archive.
## Requirements
### Requirement: Song part chord overrides must be non-empty

The song generator SHALL reject a per-part chord override that resolves
to zero chords (e.g. an empty or whitespace-only string), returning an
error before resolution. Contour resolution SHALL NOT panic when handed
an empty chord set; it SHALL return an error instead.

#### Scenario: Empty per-part chords are rejected

- **GIVEN** a valid CLI chord progression and a song whose part sets
  `chords: ""`
- **WHEN** the song is generated
- **THEN** generation returns an error identifying the offending part
- **AND** the process does not panic

#### Scenario: Contour resolution does not panic on empty chords

- **GIVEN** generation parameters whose chord list is empty and a
  pattern with a non-rest contour token
- **WHEN** the pattern is resolved
- **THEN** resolution returns an error rather than panicking

#### Scenario: Valid per-part chords still generate

- **GIVEN** a song whose part sets `chords: "Am Dm"`
- **WHEN** the song is generated
- **THEN** the part resolves against those chords and generation
  succeeds

### Requirement: Song part motif emphasis length must match its rhythm

The song generator SHALL reject a song part whose motif specifies a non-empty
`emphasis` array whose length differs from the length of the motif's `rhythm`
array, returning an error that identifies the offending part before the motif is
expanded. Motif expansion SHALL NOT panic (index out of bounds) when slicing a
part's emphasis array, even if a mismatched array reaches the expander.

#### Scenario: Mismatched motif emphasis is rejected

- **GIVEN** a song whose part `verse` has a motif with `rhythm: ["1/4", "1/4"]`,
  `contour: [root, step_up]`, and `emphasis: ["strong"]`
- **WHEN** the song is generated
- **THEN** generation returns an error identifying the part `verse`
- **AND** the process does not panic

#### Scenario: Expansion does not panic on a short emphasis array

- **GIVEN** a song part whose motif emphasis array is shorter than its rhythm
  array and whose structure includes a `return` or `truncation` transform
- **WHEN** the motif is expanded
- **THEN** expansion returns an error rather than panicking with an
  out-of-bounds slice

#### Scenario: Matching or omitted emphasis still generates

- **GIVEN** a song part whose motif emphasis array length equals its rhythm
  length, or omits emphasis entirely
- **WHEN** the song is generated
- **THEN** the part expands and generation succeeds

### Requirement: Contour tokens must be recognized during resolution

Contour resolution SHALL reject a contour token it cannot interpret, returning
an error rather than silently substituting a pitch. `resolve_pattern` SHALL
return an error when a non-rest position carries a contour token that is not a
recognized keyword, and when a `leap_up_<n>` or `leap_down_<n>` token has a
suffix that is not a valid integer.

#### Scenario: Unknown contour token is rejected

- **GIVEN** a pattern with `rhythm.hits: ["1/4"]` and `contour: [bogus]`, a
  non-empty chord list, and an instrument range
- **WHEN** the pattern is resolved
- **THEN** resolution returns an error indicating an unknown contour token

#### Scenario: Malformed leap token is rejected

- **GIVEN** a pattern with `rhythm.hits: ["1/4"]` and `contour: [leap_up_x]`, a
  non-empty chord list, and an instrument range
- **WHEN** the pattern is resolved
- **THEN** resolution returns an error indicating an invalid leap

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

### Requirement: Motif expansion must reject missing motifs and unknown transformations

Motif expansion SHALL reject a pattern it cannot expand, returning an error
rather than producing an empty or undefined result. `expand_motif` SHALL return
an error when the pattern it is given has no `motif`, and SHALL return an error
when the pattern's `structure` names a transformation that is not a recognized
keyword.

#### Scenario: Pattern without a motif is rejected

- **GIVEN** a direct pattern with `rhythm` and `contour` but no `motif`
- **WHEN** the pattern is expanded as a motif
- **THEN** expansion returns an error indicating the pattern has no motif to
  expand

#### Scenario: Unknown transformation is rejected

- **GIVEN** a motif-based pattern whose `structure` contains an unrecognized
  transform such as `bogus_xform`
- **WHEN** the pattern is expanded as a motif
- **THEN** expansion returns an error identifying the unknown transformation

