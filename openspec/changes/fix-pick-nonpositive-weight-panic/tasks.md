## 1. Reject non-positive pick weights at parse time
- [ ] 1.1 In `src/dsl/parser.rs`, `parse_repeat_block`, in the
  `line.starts_with("pick ")` branch where each `weighted_item` is
  read, after computing `weight`, return
  `Err(anyhow!("pick weight must be positive, got {weight} for '{name}'"))`
  when `weight <= 0.0`.

## 2. Harden weighted_pick as defense in depth
- [ ] 2.1 In `src/dsl/expand.rs`, `weighted_pick`, before calling
  `rng.gen_range(0.0..total)`, add a guard: if `total <= 0.0`, return
  `choices.last().unwrap().name.clone()` (choices is always non-empty
  per the grammar) instead of sampling an empty range.

## 3. Test
- [ ] 3.1 Add a unit test in `src/dsl/parser.rs` asserting
  `parse_script("repeat 1 {\n  pick [a:0]\n}\n")` returns `Err`.
- [ ] 3.2 Add a test asserting `parse_script` of a `pick [a:-1]` block
  returns `Err`.
- [ ] 3.3 Add a unit test in `src/dsl/expand.rs` calling
  `weighted_pick(&[WeightedChoice { name: "a".into(), weight: 0.0 }], &mut make_rng())`
  and asserting it returns `"a"` without panicking.
