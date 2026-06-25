# Tasks: cover the contour-token resolution error paths

## 1. resolve_pattern error-path tests (`src/generate/resolver.rs` tests module)
- [ ] 1.1 `resolve_pattern_rejects_unknown_contour_token` — build a
  `PatternFile::from_yaml` with `rhythm.hits: ["1/4"]` and `contour: [bogus]`,
  call `resolve_pattern` with `make_params(PitchClass::C, Mode::Major, &["Cmaj"], "C2-G3")`,
  and assert the result is `Err` whose message contains "Unknown contour token".
- [ ] 1.2 `resolve_pattern_rejects_invalid_leap_token` — build a
  `PatternFile::from_yaml` with `rhythm.hits: ["1/4"]` and
  `contour: [leap_up_x]`, call `resolve_pattern` with the same params, and
  assert the result is `Err` whose message contains "Invalid leap".

## 2. Confirm no regressions
- [ ] 2.1 Confirm the existing resolver tests (`test_walking_jazz_bass`,
  `test_empty_chords_errors_not_panics`, etc.) still pass alongside the new
  tests.
