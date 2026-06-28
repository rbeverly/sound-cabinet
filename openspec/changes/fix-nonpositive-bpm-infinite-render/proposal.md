# Reject a non-positive tempo before rendering

## Why

A score's tempo comes straight from the `bpm` line with no validation.
`parse_bpm` (`src/dsl/parser.rs:572-575`) parses the value as an `f64`
and `Engine::handle_command`'s `SetBpm` branch
(`src/engine/engine.rs:161-171`) stores it unconditionally as
`self.bpm`. Nothing rejects `bpm 0`.

When `self.bpm == 0.0`, `beats_to_samples`
(`src/engine/engine.rs:911-947`) computes
`total_seconds += beats_in_seg * 60.0 / seg_bpm`, i.e. a division by
zero that yields `f64::INFINITY`, then `(total_seconds * sample_rate)
as u64` saturates to `u64::MAX`. Every played event therefore gets
`end_sample == u64::MAX` (`src/engine/engine.rs:185-186`,
`duration_secs = duration_beats * 60.0 / self.bpm`).

`render_to_wav` (`src/render/wav.rs:19`) loops
`while !engine.is_finished()`, and `is_finished`
(`src/engine/engine.rs:536-538`) returns `self.schedule.is_empty()`.
Events are only dropped by `self.schedule.retain(|e| e.end_sample >
buf_end)` (`src/engine/engine.rs:523`). With `end_sample == u64::MAX`,
no event is ever dropped, the schedule never empties, and the render
loop never terminates — it writes audio to the output WAV forever.

**Harm:** a `.sc` file containing `bpm 0` and any `play … for N beats`
makes `sound-cabinet render` hang in an unbounded loop, writing an
ever-growing WAV file until the disk fills. This is a denial-of-service
crash/hang triggered by a single line in an untrusted score file.

This is the same class of defect canon already guards elsewhere — see
`score-export`'s "must reject non-finite note timings … SHALL always
terminate … rather than entering an unbounded loop" and
`score-expansion`'s "Section repeat intervals must be positive …
Expansion of a section SHALL always terminate." There is no canonical
requirement covering the core render/play engine's handling of tempo, so
this change introduces one.

### Contract change

This adds a `score-rendering` capability requirement: the engine SHALL
reject a non-positive (or non-finite) tempo during command handling,
returning an error before rendering, so rendering always terminates.
This is an observable behavior change at the CLI boundary — `bpm 0` now
produces an error instead of hanging — hence the spec lane.

## What Changes

- `Engine::handle_command`'s `SetBpm` branch rejects a `bpm` value that
  is not a finite, strictly-positive number, returning an error before
  it is stored or rendered.

## Impact

- `src/engine/engine.rs` — validation in the `SetBpm` command handler.
- New capability spec `score-rendering`.
- Operator follow-up: none.
