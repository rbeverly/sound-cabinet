# Tasks: stop sheet-music export hanging on non-finite note timings

## 1. Reject non-finite note timings in the export pipeline
- [x] 1.1 In `src/export/mod.rs::run_export`, after notes have been extracted and
  filtered and before generating LilyPond (around the `notes.is_empty()` guard
  at line 81), iterate `notes` and return
  `Err(anyhow!("Cannot export note with non-finite timing: beat {}, duration {}", n.beat, n.duration_beats))`
  for the first note whose `beat` or `duration_beats` is not finite
  (`!f64::is_finite`). This also covers `NaN`.

## 2. Make rest generation always terminate
- [x] 2.1 In `src/export/lilypond.rs::make_rests` (line 287), at the top of the
  function return an empty `Vec` immediately when `!beats.is_finite() || beats <= 0.0`,
  so the decomposition loop can never spin forever even if a non-finite gap
  reaches it. This protects both `render_note_sequence` (line 224) and
  `render_drum_sequence` (line 280).

## 3. Tests
- [x] 3.1 Add a unit test `make_rests_terminates_on_infinite_gap` in
  `src/export/lilypond.rs` asserting `make_rests(f64::INFINITY, 0.0, 4.0)`
  returns without hanging (e.g. returns an empty vector).
- [x] 3.2 Add a unit test `export_rejects_nonfinite_duration` in
  `src/export/mod.rs` that constructs an `ExtractedScore`-equivalent input path:
  build a small expanded command list containing a `Command::PlayAt` whose
  `duration_beats` is `f64::INFINITY`, run the relevant portion of `run_export`
  (or factor the finiteness check into a small testable helper), and assert it
  returns `Err` rather than hanging.
- [x] 3.3 Add a unit test confirming a normal finite score still exports
  successfully (no regression in `render_note_sequence` output for finite
  timings).
