## 1. Validate the arp `accent` value at parse time
- [ ] 1.1 In `src/engine/engine.rs`, `parse_arp_options`, in the
  `name == "accent"` branch, replace
  `opts.accent_every = Some(*v as usize)` with a check that the value
  is a positive integer: if `*v < 1.0` (covers `0` and negatives,
  which otherwise saturate to `0` on `as usize`), return
  `Err(anyhow::anyhow!("arp: 'accent' must be a positive integer"))`;
  otherwise store `Some(*v as usize)`.

## 2. Guard the use site as defense in depth
- [ ] 2.1 In `src/engine/engine.rs`, `try_handle_arp`, change the
  accent gain computation so the modulo only runs when the divisor is
  non-zero, e.g. `if let Some(n) = opts.accent_every { if n > 0 && i % n == 0 { 1.5 } else { 0.7 } } else { 1.0 }`.

## 3. Test
- [ ] 3.1 Add a unit test (in `src/engine/engine.rs` tests) that calls
  `parse_arp_options` with args representing `arp(C4, 4, accent, 0)`
  (i.e. `[Expr::Number(261.63), Expr::Number(4.0), Expr::VoiceRef("accent".into()), Expr::Number(0.0)]`)
  and asserts it returns `Err`.
- [ ] 3.2 Add a test that scheduling an arp with `accent 0` via
  `Engine::handle_command` on a `Command::PlayAt` whose expression is
  `... >> arp(C4, 4, accent, 0)` returns `Err` and does not panic.
- [ ] 3.3 Add a regression test that `accent 2` still parses to
  `accent_every == Some(2)`.
