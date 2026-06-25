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

