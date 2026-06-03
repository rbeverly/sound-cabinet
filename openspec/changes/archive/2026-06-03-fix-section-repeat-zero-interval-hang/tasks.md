## 1. Reject non-positive repeat intervals in section expansion
- [x] 1.1 In `src/dsl/expand.rs`, inside
  `ExpansionContext::expand_section`, before the
  `while beat < section.duration_beats` loop in the
  `SectionEntry::RepeatEvery` arm, return
  `Err(anyhow!("section repeat interval must be positive, got {every_beats} beats for '{name}'"))`
  when `*every_beats <= 0.0`.
- [x] 1.2 Keep the existing loop body unchanged for positive intervals.

## 2. Test
- [x] 2.1 Add a unit test `expand_rejects_zero_repeat_interval` in
  `src/dsl/expand.rs` that builds a `Script` with a `PatternDef`, a
  `SectionDef` containing
  `SectionEntry::RepeatEvery { every_beats: 0.0, .. }`, and a
  `PlaySequential` for that section, then asserts
  `expand_script(script, &mut make_rng())` returns `Err` (and does not
  hang).
- [x] 2.2 Add `expand_rejects_negative_repeat_interval` asserting the
  same for `every_beats: -1.0`.
