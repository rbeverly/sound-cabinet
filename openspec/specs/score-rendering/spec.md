# score-rendering Specification

## Purpose
TBD - created by archiving change fix-nonpositive-bpm-infinite-render. Update Purpose after archive.
## Requirements
### Requirement: Tempo must be a positive number

The render/play engine SHALL reject a tempo (`bpm`) value that is not a
finite, strictly-positive number, returning an error during command
handling before rendering. Rendering a score to audio SHALL always
terminate; a non-positive tempo SHALL NOT produce events whose end
position saturates, which would otherwise keep the render loop running
without bound.

#### Scenario: bpm 0 is rejected without hanging

- **GIVEN** a score containing `bpm 0` followed by
  `at 0 play sine(440) for 4 beats`
- **WHEN** the engine handles the `bpm` command
- **THEN** it returns an error indicating `bpm` must be a positive
  number
- **AND** the render loop does not run without bound

#### Scenario: negative bpm is rejected

- **GIVEN** a score command setting the tempo to `-120`
- **WHEN** the engine handles the `bpm` command
- **THEN** it returns an error

#### Scenario: a positive tempo still renders

- **GIVEN** a score whose tempo is a finite, strictly-positive number
- **WHEN** the engine handles the `bpm` command and renders the score
- **THEN** the tempo is recorded and rendering proceeds and terminates
  as before

