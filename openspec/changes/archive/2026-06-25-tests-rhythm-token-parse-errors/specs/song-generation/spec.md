# song-generation Specification

## ADDED Requirements

### Requirement: Rhythm notation tokens must be well-formed

Rhythm parsing SHALL reject a malformed rhythm token or time signature with a
descriptive error rather than silently mis-parsing it. `parse_rhythm` SHALL
reject a token it does not recognize, a tied token whose component is not a
`1/<n>` duration, a duration whose denominator is non-numeric, and a duration
whose denominator is not strictly positive. `parse_time_sig` SHALL reject a
string that is not exactly two `/`-separated parts, or whose numerator or
denominator is non-numeric.

#### Scenario: Unrecognized rhythm token is rejected

- **GIVEN** a rhythm hits array `["xyz"]`
- **WHEN** the rhythm is parsed
- **THEN** parsing returns an error indicating an unrecognized rhythm token

#### Scenario: Invalid tied component is rejected

- **GIVEN** a rhythm hits array `["1/4+2/8"]` whose second tied component is not
  a `1/<n>` duration
- **WHEN** the rhythm is parsed
- **THEN** parsing returns an error indicating an invalid tied duration
  component

#### Scenario: Non-numeric duration denominator is rejected

- **GIVEN** a rhythm hits array `["1/x"]`
- **WHEN** the rhythm is parsed
- **THEN** parsing returns an error indicating an invalid duration denominator

#### Scenario: Non-positive duration denominator is rejected

- **GIVEN** a rhythm hits array `["1/0"]`
- **WHEN** the rhythm is parsed
- **THEN** parsing returns an error indicating the duration denominator must be
  positive

#### Scenario: Time signature without two parts is rejected

- **GIVEN** a time signature string `"44"` with no `/`
- **WHEN** the time signature is parsed
- **THEN** parsing returns an error indicating the expected `N/N` form

#### Scenario: Non-numeric time signature parts are rejected

- **GIVEN** a time signature string `"x/4"` (or `"4/x"`)
- **WHEN** the time signature is parsed
- **THEN** parsing returns an error identifying the non-numeric numerator (or
  denominator)
