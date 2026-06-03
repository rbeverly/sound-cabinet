## 1. Validate per-part chords in the song generator
- [ ] 1.1 In `src/generate/song.rs`, `run_generate_song`, immediately
  after computing the per-part `chords` vector (the
  `if let Some(ref chord_str) = part.chords { ... }` block), add:
  if `chords.is_empty()` return
  `Err(anyhow!("song part '{part_name}': chords override is empty"))`.

## 2. Make active_chord non-panicking (defense in depth)
- [ ] 2.1 In `src/generate/resolver.rs`, change `active_chord` to
  return `Result<&Chord>`: replace `panic!("No chords provided")` with
  `return Err(anyhow!("no chords provided for contour resolution"))`,
  and wrap the existing returns in `Ok(...)`.
- [ ] 2.2 Update the call site in `resolve_pattern`
  (`let chord = active_chord(...)`) to use `?`.

## 3. Test
- [ ] 3.1 Add a unit test in `src/generate/song.rs` that builds a
  `SongFile` (via `SongFile::from_yaml`) whose single part sets
  `chords: ""` and a `GenerateConfig` with a valid `--chords` value,
  then asserts `run_generate_song(&song, &config)` returns `Err` and
  does not panic.
- [ ] 3.2 Add a unit test in `src/generate/resolver.rs` asserting
  `resolve_pattern` returns `Err` (not a panic) when `params.chords`
  is empty and the pattern has a non-rest contour token.
