## ADDED Requirements

### Requirement: Arpeggiator allocation bounded

The engine SHALL validate the arp rate and the derived step count
before sizing internal buffers. If the rate is non-finite,
non-positive, or would produce a step count above an implementation
defined upper bound (at least one million steps), the engine SHALL
return `Result::Err` from `try_handle_arp` rather than allocating a
`Vec` whose capacity request can panic or abort the process.

The engine SHALL similarly reject a non-finite or non-positive
`duration_beats` for an `arp` event before that value is multiplied
by the rate.

This requirement protects against a denial-of-service crash on
attacker-controlled `.sc` input: previously a literal such as
`arp(A4, 1e18) for 1 beat` overflowed `Vec::with_capacity` and
aborted the process.

#### Scenario: Excessive rate produces an error, not a panic
- **GIVEN** an `arp(A4, 1e18)` event scheduled with
  `duration_beats = 4.0`
- **WHEN** the engine handles the resulting `PlayAt` command
- **THEN** the command handler returns
  `Err(...)` whose message indicates the requested step count
  exceeds the maximum
- **AND** the process does not panic

#### Scenario: Zero or negative rate rejected
- **GIVEN** an `arp(A4, 0)` or `arp(A4, -1)` event
- **WHEN** the engine handles the resulting `PlayAt` command
- **THEN** the command handler returns `Err(...)` whose message
  reports that the rate must be a finite positive number

#### Scenario: Non-finite duration rejected
- **GIVEN** an `arp(A4, 4)` event whose `duration_beats` is
  `f64::INFINITY` or `f64::NAN` (e.g. a tempo change that produced
  it through prior arithmetic)
- **WHEN** the engine handles the resulting `PlayAt` command
- **THEN** the command handler returns `Err(...)` rather than
  reaching the step-vector allocation site

#### Scenario: Normal musical rates unaffected
- **GIVEN** an `arp(C4, E4, G4, 8) for 4 beats` event (32 steps)
- **WHEN** the engine handles the resulting `PlayAt` command
- **THEN** the command succeeds — the bound is enforced only above
  the documented upper limit
