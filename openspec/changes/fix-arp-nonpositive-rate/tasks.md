## 1. Validate the arp rate in parse_arp_options
- [ ] 1.1 In `src/engine/engine.rs`, `parse_arp_options`, immediately
  after the rate is extracted (the `match &args[rate_idx] { … }` block
  around lines 1222-1230), reject any rate value that is not a finite,
  strictly-positive number. For the `Expr::Number(v)` case check `*v`;
  for the `Expr::Range(start, end)` case check both `*start` and `*end`.
  On failure return
  `Err(anyhow::anyhow!("arp: rate must be a positive number"))` before
  the values are stored in `opts`.

## 2. Tests
- [ ] 2.1 Add a unit test (in the `src/engine/engine.rs` tests module)
  asserting `parse_arp_options` for `arp(C4, 0)` (i.e. args ending in
  `Expr::Number(0.0)` as the rate) returns `Err`.
- [ ] 2.2 Add a test asserting a negative rate (rate arg
  `Expr::Number(-4.0)`) returns `Err`.
- [ ] 2.3 Add a test asserting a non-finite rate
  (rate arg `Expr::Number(f64::INFINITY)`) returns `Err` and does not
  panic (guarding the `Vec::with_capacity(total_steps)` capacity
  overflow).
- [ ] 2.4 Add a regression test asserting a valid rate
  (rate arg `Expr::Number(4.0)`) still parses to
  `rate_start == 4.0` with `rate_end == None`.
- [ ] 2.5 Add a test asserting a range rate with a non-positive
  endpoint (e.g. `Expr::Range(4.0, 0.0)`) returns `Err`.
