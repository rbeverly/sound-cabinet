## 1. Validate arp rate and step count

- [x] 1.1 In `src/engine/engine.rs`, define a private module-level
  constant `MAX_ARP_STEPS: usize = 1_000_000`. Place it near the
  other arp-related constants/structs around
  `src/engine/engine.rs:1430`.
- [x] 1.2 In `parse_arp_options`
  (`src/engine/engine.rs:1487`), after extracting
  `opts.rate_start` and the optional `opts.rate_end`, return
  `Err(anyhow!("arp: rate must be a finite positive number (got {v})"))`
  when either value is not finite (`v.is_nan() || v.is_infinite()`)
  or is `<= 0.0`. The check MUST cover both the `Expr::Number` and
  `Expr::Range` branches at lines 1547–1554.
- [x] 1.3 In `try_handle_arp`
  (`src/engine/engine.rs:1078`), early in the function (after
  `extract_arp` succeeds), reject non-finite or non-positive
  `duration_beats`:
  ```rust
  if !duration_beats.is_finite() || duration_beats <= 0.0 {
      return Err(anyhow!(
          "arp: duration must be a finite positive number of beats (got {duration_beats})"
      ));
  }
  ```
- [x] 1.4 Just before the `Vec::with_capacity(total_steps)` call at
  `src/engine/engine.rs:1167`, after `total_steps` is computed,
  return an error if it exceeds `MAX_ARP_STEPS`:
  ```rust
  if total_steps > MAX_ARP_STEPS {
      return Err(anyhow!(
          "arp: requested {total_steps} steps exceeds the maximum of {MAX_ARP_STEPS} (reduce rate or duration)"
      ));
  }
  ```
- [x] 1.5 Confirm `octaves` is already bounded (see
  `is_direction_with_octave` at `src/engine/engine.rs:1460-1478`,
  which clamps the parsed value to `1..=4`). No change needed for
  that path; document it in a comment if any reviewer asks.

## 2. Tests

- [x] 2.1 Add a unit test
  `arp_rejects_excessive_rate` in `src/engine/engine.rs` (or the
  appropriate test module) that constructs an `Expr::FnCall`
  `arp(A4, 1e18)` with `duration_beats = 4.0` and asserts the
  engine returns `Err` whose message contains `arp: rate` or
  `exceeds the maximum`. The test MUST NOT panic.
- [x] 2.2 Add a unit test `arp_rejects_non_positive_rate` covering
  both `rate = 0.0` and `rate = -1.0`, asserting each returns
  `Err` whose message contains `must be a finite positive number`.
- [x] 2.3 Add a unit test
  `arp_rejects_non_finite_duration` asserting that an arp call
  with `duration_beats = f64::INFINITY` returns `Err` rather than
  reaching the `Vec::with_capacity` call.

## 3. Spec update

- [x] 3.1 Add an `## ADDED Requirements` block to
  `openspec/specs/audio-engine/spec.md` introducing
  `Requirement: Arpeggiator allocation bounded`, with scenarios for
  the rate-too-high, zero/negative-rate, and non-finite-duration
  cases. The requirement SHALL state that the engine returns
  `Result::Err` rather than panicking for any of these inputs.
