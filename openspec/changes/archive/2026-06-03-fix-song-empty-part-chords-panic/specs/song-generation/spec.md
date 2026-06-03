# Song Generation

## ADDED Requirements

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
