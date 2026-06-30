# Reject a non-positive or non-finite arp rate

## Why

`parse_arp_options` (`src/engine/engine.rs:1163-1232`) extracts the
arpeggiator rate from the score with no validation:
`Expr::Number(v) => opts.rate_start = *v` (line 1224) and the
`Expr::Range` case (lines 1225-1228). The rate is a notes-per-beat
value; the scheduler uses it both to size the step buffer and as the
divisor `1.0 / current_rate` for the inter-step interval
(`src/engine/engine.rs:785`).

Two reachable defects, both triggered by a single `arp(...)` call in an
untrusted `.sc` file:

1. **Non-positive rate → silent no-op.** With `rate <= 0`,
   `avg_rate = (rate_start + rate_end) / 2.0` is `<= 0`, so
   `total_steps = (duration_beats * avg_rate).round() as usize`
   (`src/engine/engine.rs:764`) is `0` (a negative `f64` saturates to
   `0` on `as usize`). The step loop runs zero times and the arp
   silently produces no notes — a malformed value is accepted and the
   musical result is wrong with no diagnostic.

2. **Non-finite rate → allocation panic.** A literal large enough to
   parse to an infinite `f64` (e.g. `arp(C4, E4, G4, 1e400)`) makes
   `avg_rate` infinite, so
   `total_steps = (duration_beats * avg_rate).round() as usize` is
   `f64::INFINITY as usize == usize::MAX`. The very next line,
   `Vec::with_capacity(total_steps)` (`src/engine/engine.rs:770`),
   panics with a capacity overflow / aborts the process.

**Harm:** a crafted arp rate either silently drops the arpeggio
(correctness bug) or crashes `sound-cabinet render`/`play` (panic /
denial of service) on untrusted score input.

This mirrors canon's existing arpeggiator requirement "Arp accent value
must be a positive integer … returning an error during command
handling" and `score-export`'s "must reject non-finite … timings …
before rendering". The accent path is already guarded the same way; the
rate path is not.

### Contract change

This adds an `arpeggiator` requirement that the arp rate must be a
finite, strictly-positive number, rejected during command handling.
This is an observable behavior change — a non-positive or non-finite
rate now produces an error instead of silently dropping notes or
panicking — hence the spec lane.

## What Changes

- `parse_arp_options` validates the extracted rate (both the single
  `rate_start` and the `rate_start`/`rate_end` of a range), returning an
  error when any rate value is not a finite, strictly-positive number,
  before it is stored.

## Impact

- `src/engine/engine.rs` — validation in `parse_arp_options` after the
  rate is extracted (around lines 1222-1230).
- Spec delta: `arpeggiator` capability gains the rate requirement.
- Operator follow-up: none.
