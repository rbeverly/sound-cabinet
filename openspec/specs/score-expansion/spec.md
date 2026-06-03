# score-expansion Specification

## Purpose
TBD - created by archiving change fix-pick-nonpositive-weight-panic. Update Purpose after archive.
## Requirements
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

### Requirement: Section repeat intervals must be positive

The score expander SHALL reject a section `repeat … every <n> beats`
entry whose interval `n` is not strictly greater than zero, returning
an error rather than entering an unbounded expansion loop. Expansion of
a section SHALL always terminate.

#### Scenario: Zero repeat interval is rejected

- **GIVEN** a score defining a pattern `p` and a section `s` whose only
  entry is `repeat p every 0 beats`, followed by `play s`
- **WHEN** the script is expanded
- **THEN** expansion returns an error identifying the offending repeat
  interval
- **AND** the expander does not loop forever or grow output without
  bound

#### Scenario: Negative repeat interval is rejected

- **GIVEN** a section entry `repeat p every -2 beats`
- **WHEN** the script is expanded
- **THEN** expansion returns an error

#### Scenario: Positive repeat interval still tiles the pattern

- **GIVEN** a section `s = 8 beats` whose only entry is
  `repeat p every 4 beats`
- **WHEN** the script is expanded
- **THEN** the pattern `p` is tiled at beats 0 and 4 and expansion
  succeeds

