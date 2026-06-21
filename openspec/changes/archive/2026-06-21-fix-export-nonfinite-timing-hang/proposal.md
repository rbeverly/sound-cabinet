# Sheet-music export hangs forever on non-finite note timings

## Why

The DSL `number` grammar (`src/dsl/grammar.pest`, rule
`number = @{ "-"? ~ ASCII_DIGIT+ ~ ("." ~ ASCII_DIGIT+)? }`) accepts an
arbitrarily long digit string. `parse::<f64>()` does not error on overflow — it
returns `f64::INFINITY`. So a note beat or duration in a score file can be
infinite, and nothing between parsing and export validates finiteness
(`expand_script` only guards `every_beats <= 0.0`, a different field).

The LilyPond exporter fills timing gaps with rest tokens via `make_rests`
(`src/export/lilypond.rs:287-305`):

```rust
fn make_rests(beats: f64, _cursor: f64, _beats_per_bar: f64) -> Vec<String> {
    let mut remaining = beats;
    ...
    for &d in &standard_durations {
        while remaining >= d - 0.001 {     // inf - 4.0 == inf  → never terminates
            ...
            remaining -= d;
            ...
        }
    }
    ...
}
```

When an event's `beat + duration_beats` is infinite, `render_note_sequence`
(`src/export/lilypond.rs:162-167`) computes `total_bars = (inf / bpb).ceil() as usize`
(= `usize::MAX`) and `total_beats = inf`, then the trailing fill at
`src/export/lilypond.rs:222-225` calls `make_rests(inf, ...)`. The inner
`while remaining >= d - 0.001` loop never terminates because `inf - d == inf` —
the process hangs at 100% CPU. `render_drum_sequence`
(`src/export/lilypond.rs:240-281`) has the identical defect.

### Reproduction (attacker-controlled `.sc`, `sound-cabinet export score.sc -o out.ly`)

```
at 0 play piano(440) for 999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999 beats
```

The ~270-digit duration parses to `f64::INFINITY`, flows unmodified through
`expand_script` → `extract_notes` (`src/export/extract.rs:71`) → `write_lilypond`
→ `render_note_sequence` → `make_rests`, which loops forever. A huge `at <N>`
beat triggers the same path.

### Harm

Denial of service: a single crafted score line makes `sound-cabinet export`
hang indefinitely (never returns, pinning a CPU core).

### Contract change

This introduces a new observable behavior at the export boundary: a score with
non-finite note timings is now rejected with an error instead of hanging. The
`score-export` capability is not yet specified in canon, so the invariant is
recorded as an `## ADDED` requirement under a new `score-export` capability.

## What Changes

- `run_export` validates, after extraction and filtering, that every note's
  `beat` and `duration_beats` are finite; if any is not, it returns an error
  identifying the offending value rather than proceeding to render.
- `make_rests` is hardened so it always terminates: a non-finite or
  non-positive `beats` argument yields no rests instead of looping forever
  (defense in depth, covering both the pitched and drum render paths).

## Impact

- `src/export/mod.rs` — `run_export` gains a finiteness check on note timings
  (a natural site is just before the `notes.is_empty()` guard at line 81, or
  immediately after it).
- `src/export/lilypond.rs` — `make_rests` (line 287) guards against non-finite /
  non-positive input.
- New capability spec `score-export`.
- No operator follow-up required.

### Note on related, out-of-scope resource limits

A *finite but enormous* duration (e.g. `for 1000000000000000000 beats`) would
still allocate an enormous number of rest tokens. Bounding that requires
choosing an arbitrary maximum score length and is left out of scope here; this
change addresses only the guaranteed non-termination on non-finite timings.
