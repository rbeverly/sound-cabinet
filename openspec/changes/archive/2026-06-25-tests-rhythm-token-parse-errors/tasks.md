# Tasks: cover the rhythm-notation parse error paths

## 1. parse_rhythm error-path tests (`src/generate/rhythm.rs` tests module)
- [x] 1.1 `parse_rhythm_rejects_unrecognized_token` — asserts
  `parse_rhythm(&["xyz".to_string()])` returns `Err` whose message contains
  "Unrecognized rhythm token".
- [x] 1.2 `parse_rhythm_rejects_invalid_tied_component` — asserts
  `parse_rhythm(&["1/4+2/8".to_string()])` returns `Err` whose message contains
  "Invalid tied duration component".
- [x] 1.3 `parse_rhythm_rejects_non_numeric_denominator` — asserts
  `parse_rhythm(&["1/x".to_string()])` returns `Err` whose message contains
  "Invalid duration denominator".
- [x] 1.4 `parse_rhythm_rejects_zero_denominator` — asserts
  `parse_rhythm(&["1/0".to_string()])` returns `Err` whose message contains
  "Duration denominator must be positive".

## 2. parse_time_sig error-path tests (`src/generate/rhythm.rs` tests module)
- [x] 2.1 `parse_time_sig_rejects_missing_slash` — asserts
  `parse_time_sig("44")` returns `Err` whose message contains "expected N/N".
- [x] 2.2 `parse_time_sig_rejects_non_numeric_numerator` — asserts
  `parse_time_sig("x/4")` returns `Err` whose message contains "numerator".
- [x] 2.3 `parse_time_sig_rejects_non_numeric_denominator` — asserts
  `parse_time_sig("4/x")` returns `Err` whose message contains "denominator".

## 3. Confirm no regressions
- [x] 3.1 Confirm the existing happy-path rhythm tests (`test_time_sig_parse`,
  `test_parse_rhythm_offsets`, etc.) still pass alongside the new tests.
