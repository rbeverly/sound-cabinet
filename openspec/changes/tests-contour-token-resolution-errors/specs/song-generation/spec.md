# song-generation Specification

## ADDED Requirements

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
