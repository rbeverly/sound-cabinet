# song-generation Specification

## ADDED Requirements

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
