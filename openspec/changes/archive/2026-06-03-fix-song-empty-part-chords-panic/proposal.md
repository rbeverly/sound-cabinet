# Fix `panic!` when a song part defines empty chords

## Why

The song generator lets each part override the chord progression via a
YAML field. In `src/generate/song.rs:83-90`:

```rust
let chords = if let Some(ref chord_str) = part.chords {
    chord_str
        .split_whitespace()
        .map(|c| Chord::parse(c))
        .collect::<Result<Vec<_>>>()?
} else {
    default_chords.clone()
};
```

The top-level guard only validates `default_chords`
(`src/generate/song.rs:45-47`); the per-part override path has no
emptiness check. A song YAML whose part sets `chords: ""` (or a
whitespace-only string) makes `split_whitespace()` yield nothing, so
`chords` is an empty `Vec` with no error.

That empty vector flows into `resolve_pattern`, which calls
`active_chord` (`src/generate/resolver.rs:103-105`):

```rust
fn active_chord<'a>(chords: &'a [Chord], beat: f64, bar_beats: f64) -> &'a Chord {
    if chords.is_empty() {
        panic!("No chords provided");
    }
    ...
}
```

Any part whose motif resolves a non-rest contour token (e.g. `root`)
hits `active_chord` with an empty slice and the program panics/aborts.

Reproducing file (run via `sound-cabinet generate --pattern song.yaml
--key C --mode major --chords "Cmaj" --voice mel`):

```yaml
name: Evil
time: "4/4"
parts:
  verse:
    motif:
      rhythm: ["1/4"]
      contour: [root]
    chords: ""
arrangement: [verse]
```

Even though a valid `--chords` is supplied on the CLI, the empty
per-part override bypasses the check and triggers the panic.

**Harm:** crash (panic/abort) on attacker-controlled or mistyped song
YAML — denial of service for the `generate` subcommand.

## What Changes

- In `src/generate/song.rs`, `run_generate_song`, after resolving the
  per-part `chords`, return an `Err` if the resulting vector is empty,
  matching the existing default-chords guard.
- As defense in depth, change `active_chord` in
  `src/generate/resolver.rs` to return a `Result<&Chord>` (propagating
  an error) instead of calling `panic!`, and have `resolve_pattern`
  propagate it. (Callers already return `Result`.)

## Impact

- Affected code: `src/generate/song.rs` (`run_generate_song`),
  `src/generate/resolver.rs` (`active_chord`, `resolve_pattern`).
- Behavior change: a song part with empty/whitespace chords now yields
  a clear error instead of panicking. Songs with valid chords are
  unaffected.
- No public API or data-format change.
