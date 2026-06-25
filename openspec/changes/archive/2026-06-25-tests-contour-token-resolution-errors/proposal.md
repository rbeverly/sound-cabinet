# Cover the contour-token resolution error paths

## Why

`resolve_token` in `src/generate/resolver.rs:137-243` maps each contour token to
a pitch. Pattern validation (`PatternFile::validate`) never checks token
spelling, so a malformed contour token flows straight into `resolve_token`,
which returns an error. Two such reachable error branches are untested:

- the catch-all "Unknown contour token" (line 241) — reached by any contour
  token that is not a known keyword (e.g. `bogus`) — **untested**.
- "Invalid leap" (lines 226 and 236) — reached when a `leap_up_<n>` /
  `leap_down_<n>` token has a non-numeric suffix (e.g. `leap_up_x`) —
  **untested**.

The existing resolver tests cover only the happy paths and the empty-chords
case. These two branches are reachable through the public `resolve_pattern`
entry point (a non-rest hit with a bad contour token and a non-empty chord
list), and the code already returns the errors; canon does not specify them.

The `cursor.ok_or_else(...)` "used before any note" branches in the same `match`
are deliberately excluded: `resolve_pattern` always seeds the cursor from the
chord root before calling `resolve_token`, so those branches are unreachable
through the public API and are pure defense in depth.

### Contract change

This adds a `## ADDED` requirement under `song-generation` stating that contour
resolution rejects unrecognized tokens and malformed leap tokens. No production
code changes — `resolve_token` already returns these errors; only tests and the
canonical requirement are added.

## What Changes

- Add a `resolve_pattern` test asserting an unknown contour token surfaces an
  error.
- Add a `resolve_pattern` test asserting a `leap_up_<non-numeric>` token surfaces
  an error.

## Impact

- `src/generate/resolver.rs` — new `#[cfg(test)]` functions in the existing
  `tests` module (reusing the `make_params` helper). No production code changes.
- A new requirement under the `song-generation` capability documenting the
  already-implemented contour-token validation.
- No operator follow-up required.
