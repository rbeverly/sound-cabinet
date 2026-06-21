# score-export Specification

## Purpose
TBD - created by archiving change fix-export-nonfinite-timing-hang. Update Purpose after archive.
## Requirements
### Requirement: Sheet-music export must reject non-finite note timings

Sheet-music export SHALL reject a score in which any note's beat position or
duration is not a finite number (i.e. is infinite or NaN), returning an error
that identifies the offending value before rendering. Rest generation that fills
timing gaps SHALL always terminate; it SHALL produce no rests when handed a
non-finite or non-positive gap rather than entering an unbounded loop.

#### Scenario: Non-finite duration is rejected without hanging

- **GIVEN** a score whose only event is `at 0 play piano(440) for <N> beats`,
  where `<N>` is a digit string large enough to parse to an infinite `f64`
- **WHEN** the score is exported to LilyPond
- **THEN** export returns an error identifying the non-finite timing
- **AND** the process does not hang

#### Scenario: Rest fill terminates on a non-finite gap

- **GIVEN** an infinite gap value passed to the rest-decomposition routine
- **WHEN** rests are generated for that gap
- **THEN** the routine returns without looping forever, producing no rests for
  the non-finite gap

#### Scenario: A finite score still exports

- **GIVEN** a score whose note beats and durations are all finite
- **WHEN** the score is exported
- **THEN** rests fill the gaps as before and export succeeds

