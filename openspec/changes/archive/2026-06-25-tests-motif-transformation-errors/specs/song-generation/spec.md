# song-generation Specification

## ADDED Requirements

### Requirement: Motif expansion must reject missing motifs and unknown transformations

Motif expansion SHALL reject a pattern it cannot expand, returning an error
rather than producing an empty or undefined result. `expand_motif` SHALL return
an error when the pattern it is given has no `motif`, and SHALL return an error
when the pattern's `structure` names a transformation that is not a recognized
keyword.

#### Scenario: Pattern without a motif is rejected

- **GIVEN** a direct pattern with `rhythm` and `contour` but no `motif`
- **WHEN** the pattern is expanded as a motif
- **THEN** expansion returns an error indicating the pattern has no motif to
  expand

#### Scenario: Unknown transformation is rejected

- **GIVEN** a motif-based pattern whose `structure` contains an unrecognized
  transform such as `bogus_xform`
- **WHEN** the pattern is expanded as a motif
- **THEN** expansion returns an error identifying the unknown transformation
