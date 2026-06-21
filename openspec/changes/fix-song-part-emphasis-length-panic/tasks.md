# Tasks: reject song-part motif emphasis whose length mismatches its rhythm

## 1. Validate motif emphasis length for song parts
- [ ] 1.1 In `src/generate/pattern.rs`, inside `SongFile::validate` (the loop
  over `self.parts` at lines 248-259), add a check mirroring the
  `PatternFile::validate` motif check (lines 163-170): if
  `!part.motif.emphasis.is_empty() && part.motif.emphasis.len() != part.motif.rhythm.len()`,
  return `Err(anyhow!("Song '{}', part '{}': motif emphasis has {} entries but rhythm has {}", self.name, name, part.motif.emphasis.len(), part.motif.rhythm.len()))`.

## 2. Defense in depth in the expander
- [ ] 2.1 In `src/generate/motif.rs`, harden against a short emphasis array
  reaching expansion. Either change `motif_emphasis` (lines 467-484) to always
  return a vector whose length equals `motif.rhythm.len()` (pad with a default
  such as `"medium"` / truncate as needed), or guard the slices at the `return`
  site (line 274) and `truncation` site (line 220) to clamp the slice end to the
  emphasis length. The result must never index past the emphasis vector.

## 3. Tests
- [ ] 3.1 Add a unit test `test_song_part_short_emphasis_errors_not_panics` in
  `src/generate/song.rs` that loads a song YAML whose part motif has
  `rhythm: ["1/4", "1/4"]`, `contour: [root, step_up]`,
  `emphasis: [strong]` and `structure: [return]`, runs `run_generate_song` with
  a valid config, and asserts the result is `Err` whose message contains the
  part name — and that the process does not panic.
- [ ] 3.2 Add a unit test in `src/generate/pattern.rs` asserting
  `SongFile::from_yaml` returns `Err` for a part whose motif emphasis length
  differs from its rhythm length.
- [ ] 3.3 Confirm existing song tests (`test_generate_song`,
  `test_empty_part_chords_errors_not_panics`) still pass.
