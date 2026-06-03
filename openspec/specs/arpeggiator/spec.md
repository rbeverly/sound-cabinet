# arpeggiator Specification

## Purpose
TBD - created by archiving change fix-arp-accent-zero-divisor-panic. Update Purpose after archive.
## Requirements
### Requirement: Arp accent value must be a positive integer

The arpeggiator SHALL reject an `accent` option value that is not a
positive integer (i.e. less than 1), returning an error during command
handling. The arpeggiator SHALL NOT compute `step_index % accent_every`
when `accent_every` is zero.

#### Scenario: accent 0 is rejected without panicking

- **GIVEN** a score event `at 0 play sine(440) >> arp(C4, 4, accent, 0) for 4 beats`
- **WHEN** the engine handles the play command
- **THEN** it returns an error indicating `accent` must be a positive
  integer
- **AND** the process does not panic

#### Scenario: negative accent is rejected

- **GIVEN** an arp option `accent, -1`
- **WHEN** the engine handles the play command
- **THEN** it returns an error

#### Scenario: positive accent is honored

- **GIVEN** an arp option `accent, 2`
- **WHEN** the arp options are parsed
- **THEN** the accent interval is recorded as 2 and scheduling
  succeeds without panicking

