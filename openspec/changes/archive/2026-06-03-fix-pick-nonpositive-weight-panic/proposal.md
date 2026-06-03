# Fix empty-range panic from `pick` with non-positive weights

## Why

A `repeat { ... }` block may contain a weighted `pick` such as
`pick [verse:3, chorus:1]`. The chosen item is selected by
`weighted_pick` in `src/dsl/expand.rs:305-318`:

```rust
fn weighted_pick(choices: &[WeightedChoice], rng: &mut impl Rng) -> String {
    let total: f64 = choices.iter().map(|c| c.weight).sum();
    let mut r = rng.gen_range(0.0..total);
    ...
}
```

Weights come from the `.sc` file. The grammar
(`src/dsl/grammar.pest:76`, `weighted_item = ident ~ (":" ~ number)?`)
allows `0` and negative weights, and `parse_repeat_block`
(`src/dsl/parser.rs:514-522`) stores them verbatim. If the weights sum
to `0` (e.g. `pick [a:0]`) or are negative (e.g. `pick [a:-1]`), then
`total <= 0.0` and `rng.gen_range(0.0..total)` is called on an empty or
invalid range, which panics ("cannot sample empty range" / low >= high),
aborting the program.

Reproducing score:

```
pattern a = 4 beats
  at 0 play sine(440) for 1 beats
repeat 1 {
  pick [a:0]
}
play a
```

`weighted_pick` is reached from `expand_script`, so this crashes
`render`, `play`, `watch`, and `export`.

**Harm:** crash (panic/abort) on attacker-controlled or mistyped `.sc`
input — denial of service.

## What Changes

- In `src/dsl/parser.rs`, `parse_repeat_block`, when parsing each
  `weighted_item`, reject a weight that is not strictly positive,
  returning an error. Because the grammar requires at least one item
  and the default weight is `1.0`, this guarantees the weight sum is
  strictly positive.
- As defense in depth, in `src/dsl/expand.rs::weighted_pick`, if
  `total <= 0.0` fall back to returning the last choice instead of
  calling `gen_range` on an empty range.

## Impact

- Affected code: `src/dsl/parser.rs` (`parse_repeat_block`),
  `src/dsl/expand.rs` (`weighted_pick`).
- Behavior change: `pick` items with weight `<= 0` now produce a clear
  parse error instead of panicking. Valid positive weights are
  unaffected.
- No public API or data-format change.
