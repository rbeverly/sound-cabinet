# Cover the motif-expansion error paths

## Why

`src/generate/motif.rs` expands a motif-based pattern into a full
rhythm/contour/emphasis pattern. Two of its error branches are reachable through
the public `expand_motif` entry point but untested:

- `expand_motif` "has no motif to expand" (lines 16-19) — reached when
  `expand_motif` is handed a pattern whose `motif` is `None` (e.g. a direct
  rhythm+contour pattern) — **untested**.
- `expand_transform` "Unknown transformation" (line 314) — reached when a
  pattern's `structure` names a transform that is not one of the recognized
  keywords (e.g. `bogus_xform`) — **untested**. `structure` entries are not
  validated when the pattern is loaded, so a bad transform reaches the expander.

The existing motif tests cover only the happy paths and the recognized
transforms. The code already returns both errors; canon does not specify them.

### Contract change

This adds a `## ADDED` requirement under `song-generation` stating that motif
expansion rejects a pattern with no motif and a structure naming an unknown
transformation. No production code changes — `expand_motif` /
`expand_transform` already return these errors; only tests and the canonical
requirement are added.

## What Changes

- Add an `expand_motif` test asserting a motif-less pattern surfaces an error.
- Add an `expand_motif` test asserting a structure with an unknown transform
  surfaces an error.

## Impact

- `src/generate/motif.rs` — new `#[cfg(test)]` functions in the existing `tests`
  module. No production code changes.
- A new requirement under the `song-generation` capability documenting the
  already-implemented motif-expansion validation.
- No operator follow-up required.
