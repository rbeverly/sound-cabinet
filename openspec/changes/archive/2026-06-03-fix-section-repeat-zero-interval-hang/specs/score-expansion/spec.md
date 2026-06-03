# Score Expansion

## ADDED Requirements

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
