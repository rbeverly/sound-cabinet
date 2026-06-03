# Fix remainder-by-zero panic from arp `accent 0`

## Why

The arpeggiator accepts an `accent <n>` option that boosts every Nth
step. The value is parsed in
`src/engine/engine.rs:1181-1182` as:

```rust
if let Some(Expr::Number(v)) = option_args.get(oi) {
    opts.accent_every = Some(*v as usize);
}
```

and used in the scheduling loop at `src/engine/engine.rs:872-876`:

```rust
let accent_gain = if let Some(n) = opts.accent_every {
    if i % n == 0 { 1.5 } else { 0.7 }
} else {
    1.0
};
```

Nothing forbids `n == 0`. A `.sc` score containing `accent 0` (or a
negative value, which saturates to `0` via `*v as usize`) sets
`accent_every = Some(0)`, and the first scheduled step evaluates
`i % 0`. Integer remainder by zero is a guaranteed panic in Rust in all
build profiles ("attempt to calculate the remainder with a divisor of
zero"), aborting the whole program.

Reproducing score:

```
at 0 play sine(440) >> arp(C4, 4, accent, 0) for 4 beats
```

(`C4` is a note, `4` is the rate, `accent 0` the option.) `arp(...)` is
handled in `try_handle_arp`, reached from `Command::PlayAt` in both
`handle_command` and `handle_command_relative`, so this crashes
`render`, `play`, `watch`, and `stream`.

**Harm:** crash (panic/abort) on attacker-controlled or mistyped `.sc`
input — a denial of service that takes down the renderer/player.

## What Changes

- In `src/engine/engine.rs`, `parse_arp_options`, validate the `accent`
  value: require it to be a positive integer (`>= 1`) and return an
  `Err` for `0`, negative, or non-integer values, instead of silently
  storing `Some(0)`.
- As defense in depth, guard the use site in `try_handle_arp` so the
  accent branch only runs when `n > 0`.

## Impact

- Affected code: `src/engine/engine.rs` (`parse_arp_options`,
  `try_handle_arp`).
- Behavior change: `arp(..., accent, 0)` (and negative accent) now
  return a clear error instead of panicking. Valid `accent` values
  (>= 1) behave as before.
- No public API or data-format change.
