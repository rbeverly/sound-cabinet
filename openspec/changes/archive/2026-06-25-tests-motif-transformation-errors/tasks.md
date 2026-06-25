# Tasks: cover the motif-expansion error paths

## 1. expand_motif error-path tests (`src/generate/motif.rs` tests module)
- [x] 1.1 `expand_motif_errors_when_no_motif` — build a `PatternFile::from_yaml`
  direct pattern (`rhythm.hits: ["1/4"]`, `contour: [root]`, no `motif`), call
  `expand_motif(&pattern, (4, 4))`, and assert the result is `Err` whose message
  contains "has no motif to expand".
- [x] 1.2 `expand_motif_rejects_unknown_transformation` — build a
  `PatternFile::from_yaml` with a valid motif (`rhythm: ["1/4", "1/4"]`,
  `contour: [root, step_up]`) and `structure: [bogus_xform]`, call
  `expand_motif(&pattern, (4, 4))`, and assert the result is `Err` whose message
  contains "Unknown transformation".

## 2. Confirm no regressions
- [x] 2.1 Confirm the existing motif tests (`test_expand_simple_motif`,
  `test_expand_with_default_structure`, etc.) still pass alongside the new
  tests.
