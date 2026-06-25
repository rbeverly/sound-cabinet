# Cover the rhythm-notation parse error paths

## Why

`src/generate/rhythm.rs` parses every rhythm token and time signature that
reaches generation, yet all 17 of its tests are happy-path. None of its six
error branches is exercised:

- `parse_one_hit` (via `parse_rhythm`) "Unrecognized rhythm token" (line 102) —
  **untested**.
- `parse_one_hit` "Invalid tied duration component" (line 90) — **untested**.
- `parse_duration_value` "Invalid duration denominator" (line 116) —
  **untested**.
- `parse_duration_value` "Duration denominator must be positive" (line 119) —
  **untested** (a `1/0` token).
- `parse_time_sig` "Invalid time signature: expected N/N" (line 138) —
  **untested**.
- `parse_time_sig` "Invalid time signature numerator/denominator" (lines
  142-145) — **untested**.

These tokens come straight from user-authored YAML, so the error branches are
the contract for malformed input. The code already returns these errors; canon
does not specify them.

### Contract change

This adds a `## ADDED` requirement under `song-generation` stating that rhythm
notation tokens and time signatures must be well-formed and are rejected with a
descriptive error otherwise. No production code changes — the parser already
behaves this way; only tests and the canonical requirement are added.

## What Changes

- Add `parse_rhythm` error-path tests for an unrecognized token, an invalid tied
  component, a non-numeric denominator, and a zero denominator.
- Add `parse_time_sig` error-path tests for a missing `/`, a non-numeric
  numerator, and a non-numeric denominator.

## Impact

- `src/generate/rhythm.rs` — new `#[cfg(test)]` functions in the existing
  `tests` module. No production code changes.
- A new requirement under the `song-generation` capability documenting the
  already-implemented rhythm-parse validation.
- No operator follow-up required.
