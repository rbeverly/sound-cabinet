## Why

The arpeggiator handler in
`Engine::try_handle_arp` (`src/engine/engine.rs:1078`) accepts an
unbounded rate parameter from the DSL and uses it to size a `Vec`:

```rust
// src/engine/engine.rs:1155-1167
let rate_start = opts.rate_start;
let rate_end   = opts.rate_end.unwrap_or(rate_start);
...
let avg_rate    = (rate_start + rate_end) / 2.0;
let total_steps = (duration_beats * avg_rate).round() as usize;
...
let mut beat_offsets: Vec<f64> = Vec::with_capacity(total_steps);
```

`opts.rate_start` is parsed directly from an `Expr::Number` literal
in `parse_arp_options` (`src/engine/engine.rs:1547-1554`) with no
validation. The grammar accepts arbitrarily large numeric literals
(`number = @{ "-"? ~ ASCII_DIGIT+ ~ ("." ~ ASCII_DIGIT+)? }`).
Likewise the `for N beats` duration is parsed by `at_stmt` with no
upper bound. A score line such as

```
at 0 play arp(A4, 1e18) for 4 beats
```

drives `total_steps` to `≈ 4e18`. `Vec::with_capacity` rejects any
allocation request whose byte size exceeds `isize::MAX`
(`capacity overflow`); short of that, the allocator returns an error
and Rust aborts. Either way the process crashes via panic on
attacker-controlled input rather than returning an error to the
caller.

The harm is denial-of-service: any user who runs
`sound-cabinet render <file>.sc`, `sound-cabinet play <file>.sc`,
`sound-cabinet freeze <file>.sc`, or `sound-cabinet export <file>.sc`
against a crafted score gets an immediate panic instead of a normal
parse / engine error message. The fix is to validate the arp rate
and computed step count at engine time and return a proper
`anyhow::Error` if they fall outside a musically meaningful range.

## What Changes

- Reject non-finite or non-positive rates inside
  `parse_arp_options` / `try_handle_arp`. `rate_start` and (when
  present) `rate_end` MUST be finite and strictly greater than zero.
- Enforce an upper bound on the computed step count. Pick a constant
  (e.g. `MAX_ARP_STEPS = 1_000_000`, which is already orders of
  magnitude beyond any reasonable musical use) and refuse to size
  `beat_offsets` larger than that. Return
  `anyhow::Error` (`arp: rate too high (...)`) instead.
- Apply the same finite / non-negative check to `duration_beats` at
  the entry to `try_handle_arp` so a `for inf beats` value cannot
  bypass the rate check by multiplying through.
- Update the existing arp-related unit tests with two new cases:
  one asserting that a huge rate yields `Err` (and does NOT panic),
  one asserting that a zero / negative rate yields `Err`.

## Impact

- `src/engine/engine.rs` — the validation in `parse_arp_options`
  and `try_handle_arp`, plus the new tests.
- `openspec/specs/audio-engine/spec.md` — a new requirement under
  the arpeggiator capability locking in the bounded-step invariant.
- No grammar change. Scores with rates in the normal musical range
  (1–64 notes per beat) are unaffected.
