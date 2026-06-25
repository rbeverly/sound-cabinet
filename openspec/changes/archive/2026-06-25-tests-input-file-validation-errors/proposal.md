# Cover the input-file structural-validation error paths

## Why

The three `validate` routines that guard `sound-cabinet`'s YAML inputs enforce
many invariants, but only a few of their error branches are exercised by tests.
The code already implements every check below; canon does not yet specify them,
and the failure branches are untested.

`PatternFile::validate` (`src/generate/pattern.rs:101-177`):

- "must have either rhythm+contour or motif" (lines 106-111) — **untested**.
- direct-pattern emphasis/rhythm length mismatch (lines 124-131) — **untested**
  (only the rhythm/contour mismatch at 114-122 is covered by
  `test_mismatched_lengths_rejected`).
- contour-is-rest / rhythm-is-not-rest alignment (lines 144-149) — **untested**
  (only the opposite direction is covered by `test_rest_mismatch_rejected`).
- motif rhythm/contour length mismatch (lines 154-162) — **untested**.
- motif emphasis/rhythm length mismatch (lines 163-170) — **untested**.

`SongFile::validate` (`src/generate/pattern.rs:228-273`):

- empty `parts` (lines 229-231) — **untested**.
- empty `arrangement` (lines 232-237) — **untested**.
- per-part motif rhythm/contour mismatch (lines 250-258) — **untested**
  (the per-part emphasis mismatch at 259-269 and the undefined-arrangement-entry
  branch are already covered).

`DrumPattern::validate` (`src/generate/pattern.rs:318-332`):

- empty `voices` (lines 319-321) — **untested**.
- drum voice emphasis/rhythm mismatch (lines 322-329) — **untested**.

### Contract change

These are real, observable rejections at the `generate` boundary (a malformed
pattern/song/drum YAML is reported as an error rather than producing garbage or
panicking downstream), but canon specifies none of them. This change records the
invariants as `## ADDED` requirements under the `song-generation` capability and
adds the tests that assert them. No production code changes — the checks already
exist.

## What Changes

- Add `PatternFile::validate` error-path tests: missing shape, direct emphasis
  mismatch, reverse rest-alignment mismatch, motif rhythm/contour mismatch, and
  motif emphasis mismatch.
- Add `SongFile::validate` error-path tests: empty parts, empty arrangement, and
  per-part motif rhythm/contour mismatch.
- Add `DrumPattern::validate` error-path tests: empty voices and drum emphasis
  mismatch.

## Impact

- `src/generate/pattern.rs` — new `#[cfg(test)]` functions in the existing
  `tests` module. No production code changes.
- New requirements under the `song-generation` capability documenting the
  already-implemented input-validation invariants.
- No operator follow-up required.
