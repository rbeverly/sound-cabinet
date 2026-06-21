# Reject song-part motif emphasis whose length mismatches its rhythm

## Why

`SongFile::validate` (`src/generate/pattern.rs:248-259`) checks that each song
part's motif has matching `rhythm` and `contour` lengths, but — unlike
`PatternFile::validate` (`src/generate/pattern.rs:163-170`) — it never validates
the motif's `emphasis` length. A song part may therefore carry a non-empty
`emphasis` array shorter than its `rhythm`.

During expansion, several transforms slice the emphasis array by a length
derived from the rhythm:

- `src/generate/motif.rs:274` (`return` transform):
  `let mut emph: Vec<String> = motif_emphasis(motif)[..n].to_vec();` with
  `n = motif.rhythm.len().min(2)`.
- `src/generate/motif.rs:220` (`truncation` transform):
  `let emph = &motif_emphasis(motif)[..n];` with `n = (motif.rhythm.len() + 1) / 2`.

`motif_emphasis` (`src/generate/motif.rs:467-484`) returns `motif.emphasis.clone()`
verbatim when emphasis is non-empty, so its length can be less than `n`. Slicing
`[..n]` then panics: *"range end index N out of range for slice of length M"*.

### Reproduction (attacker-controlled YAML, `sound-cabinet generate --pattern song.yaml ...`)

```yaml
name: Evil
time: "4/4"
parts:
  verse:
    motif:
      rhythm: ["1/4", "1/4"]
      contour: [root, step_up]
      emphasis: [strong]      # length 1, rhythm length 2
    structure: [return]
arrangement: [verse]
```

`SongFile::validate` accepts this (rhythm and contour both length 2; emphasis
unchecked). `run_generate_song` (`src/generate/song.rs:80`) calls
`expand_motif`, the `return` transform computes `n = 2`, and
`motif_emphasis(motif)[..2]` on a length-1 vector panics.

### Harm

Denial of service: a crafted song YAML crashes `sound-cabinet generate` with an
index-out-of-bounds panic. This is the same class of defect already fixed for
empty per-part chords (`fix-song-empty-part-chords-panic`), in the same code
path.

### Contract change

This adds a new observable rejection at the `generate` boundary: a song part
whose motif emphasis length disagrees with its rhythm length is now reported as
an error before expansion, where previously it either panicked (via `return` /
`truncation`) or silently produced inconsistent output. Canon does not yet
specify any motif-emphasis invariant, so this is recorded as an `## ADDED`
requirement under the `song-generation` capability rather than a behavior-
preserving correction.

## What Changes

- `SongFile::validate` rejects any part whose motif specifies a non-empty
  `emphasis` array whose length differs from the motif's `rhythm` length,
  returning an error that names the offending part — mirroring the existing
  `PatternFile::validate` motif check.
- Defense in depth: `motif_emphasis` (or the slice sites) is hardened so a
  shorter emphasis array can never cause an out-of-bounds slice, even if a
  mismatched array reaches expansion.

## Impact

- `src/generate/pattern.rs` — `SongFile::validate` gains the emphasis-length
  check.
- `src/generate/motif.rs` — defensive normalization in `motif_emphasis` (and/or
  the `return`/`truncation` slice sites) so expansion never panics on a short
  emphasis array.
- New capability requirement under `song-generation`.
- No operator follow-up required.
