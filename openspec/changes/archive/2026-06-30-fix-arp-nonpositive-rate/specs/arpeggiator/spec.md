# Arpeggiator

## ADDED Requirements

### Requirement: Arp rate must be a positive number

The arpeggiator SHALL reject a `rate` value (the single rate, or either
endpoint of a rate range) that is not a finite, strictly-positive
number, returning an error during command handling. The arpeggiator
SHALL NOT size its step buffer from a non-finite rate (which would
saturate the step count to `usize::MAX` and overflow the allocation),
and SHALL NOT silently schedule zero steps for a non-positive rate.

#### Scenario: rate 0 is rejected

- **GIVEN** a score event `at 0 play sine(440) >> arp(C4, 0) for 4 beats`
- **WHEN** the engine handles the play command
- **THEN** it returns an error indicating the rate must be a positive
  number
- **AND** the arp does not silently schedule zero steps

#### Scenario: negative rate is rejected

- **GIVEN** an arp rate argument of `-4`
- **WHEN** the engine handles the play command
- **THEN** it returns an error

#### Scenario: non-finite rate is rejected without panicking

- **GIVEN** an arp rate argument that parses to an infinite `f64`
  (a digit string large enough to overflow `f64`)
- **WHEN** the engine handles the play command
- **THEN** it returns an error identifying the non-finite rate
- **AND** the process does not panic on the step-buffer allocation

#### Scenario: positive rate is honored

- **GIVEN** an arp rate argument of `4`
- **WHEN** the arp options are parsed
- **THEN** the rate is recorded as 4 and scheduling succeeds without
  panicking
