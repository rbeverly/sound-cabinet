## 1. Reject a non-positive tempo in the command handler
- [x] 1.1 In `src/engine/engine.rs`, in `Engine::handle_command`'s
  `Command::SetBpm { bpm, at_beat }` branch (currently around lines
  161-171), at the top of the branch return an error when `bpm` is not
  a finite, strictly-positive number: e.g.
  `if !(bpm.is_finite() && bpm > 0.0) { return Err(anyhow::anyhow!("bpm must be a positive number")); }`,
  before any `self.tempo_map` mutation or `self.bpm = bpm` assignment.

## 2. Tests
- [x] 2.1 Add a unit test (in the `src/engine/engine.rs` tests module)
  asserting that `Engine::handle_command(Command::SetBpm { bpm: 0.0, at_beat: None })`
  returns `Err` and does not modify the engine's tempo.
- [x] 2.2 Add a test asserting a negative tempo
  (`Command::SetBpm { bpm: -120.0, at_beat: None }`) returns `Err`.
- [x] 2.3 Add a regression test asserting a valid tempo
  (`Command::SetBpm { bpm: 120.0, at_beat: None }`) still returns `Ok`
  and that `beats_to_samples(1.0) == 22050`.
- [x] 2.4 Add a test that builds an `Engine`, handles
  `SetBpm { bpm: 0.0, .. }` then a `PlayAt` event, and asserts the
  `SetBpm` command surfaced the error (so no event with a saturated
  `end_sample` is ever scheduled and the render loop cannot hang).
