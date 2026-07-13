# Score Expansion

## ADDED Requirements

### Requirement: Weighted pick weights must be positive

A weighted `pick` in a `repeat` block SHALL reject any choice whose
weight is not strictly greater than zero, returning a parse error.
Weighted selection SHALL NOT sample an empty probability range, even
if a non-positive total ever reaches the selector.

#### Scenario: Zero weight is rejected

- **GIVEN** a script `repeat 1 {\n  pick [a:0]\n}`
- **WHEN** the script is parsed
- **THEN** parsing returns an error identifying the offending weight
- **AND** no panic occurs

#### Scenario: Negative weight is rejected

- **GIVEN** a pick item `[a:-1]`
- **WHEN** the script is parsed
- **THEN** parsing returns an error

#### Scenario: Selector tolerates a non-positive total defensively

- **GIVEN** a single choice whose weight is `0.0` passed directly to the
  weighted selector
- **WHEN** a choice is selected
- **THEN** the selector returns that choice without panicking
