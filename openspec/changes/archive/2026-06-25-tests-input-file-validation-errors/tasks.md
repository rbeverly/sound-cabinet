# Tasks: cover the input-file structural-validation error paths

## 1. PatternFile::validate error-path tests (`src/generate/pattern.rs` tests module)
- [x] 1.1 `pattern_file_requires_rhythm_contour_or_motif` — asserts
  `PatternFile::from_yaml` of a file with `name`, `type`, `time` but no
  `rhythm`, no `contour`, and no `motif` returns `Err` whose message contains
  "must have either rhythm+contour or motif".
- [x] 1.2 `pattern_file_rejects_direct_emphasis_length_mismatch` — asserts
  `PatternFile::from_yaml` with `rhythm.hits: ["1/4", "1/4"]`,
  `contour: [root, step_up]`, `emphasis: [strong]` returns `Err` whose message
  contains "emphasis has 1 entries but rhythm has 2".
- [x] 1.3 `pattern_file_rejects_contour_rest_without_rhythm_rest` — asserts
  `PatternFile::from_yaml` with `rhythm.hits: ["1/4", "1/4"]` and
  `contour: [root, "~"]` returns `Err` whose message contains
  "contour position 2 is '~'".
- [x] 1.4 `pattern_file_rejects_motif_rhythm_contour_mismatch` — asserts
  `PatternFile::from_yaml` with `motif.rhythm: ["1/4", "1/4"]` and
  `motif.contour: [root]` returns `Err` whose message contains
  "motif rhythm has 2 entries but motif contour has 1".
- [x] 1.5 `pattern_file_rejects_motif_emphasis_length_mismatch` — asserts
  `PatternFile::from_yaml` with `motif.rhythm: ["1/4", "1/4"]`,
  `motif.contour: [root, step_up]`, `motif.emphasis: [strong]` returns `Err`
  whose message contains "motif emphasis has 1 entries but motif rhythm has 2".

## 2. SongFile::validate error-path tests (`src/generate/pattern.rs` tests module)
- [x] 2.1 `song_file_rejects_empty_parts` — asserts `SongFile::from_yaml` with
  `parts: {}` and `arrangement: [verse]` returns `Err` whose message contains
  "must have at least one part".
- [x] 2.2 `song_file_rejects_empty_arrangement` — asserts `SongFile::from_yaml`
  with one valid part and `arrangement: []` returns `Err` whose message
  contains "arrangement must have at least one entry".
- [x] 2.3 `song_file_rejects_part_motif_rhythm_contour_mismatch` — asserts
  `SongFile::from_yaml` with a part `verse` whose motif has
  `rhythm: ["1/4", "1/4"]` and `contour: [root]`, and `arrangement: [verse]`,
  returns `Err` whose message contains "verse" and
  "motif rhythm has 2 entries but contour has 1".

## 3. DrumPattern::validate error-path tests (`src/generate/pattern.rs` tests module)
- [x] 3.1 `drum_pattern_rejects_empty_voices` — asserts `DrumPattern::from_yaml`
  with `voices: []` returns `Err` whose message contains
  "must have at least one voice".
- [x] 3.2 `drum_pattern_rejects_emphasis_length_mismatch` — asserts
  `DrumPattern::from_yaml` with a voice whose `rhythm: ["1/4", "1/4"]` and
  `emphasis: [strong]` returns `Err` whose message contains
  "emphasis has 1 entries but rhythm has 2".

## 4. Confirm no regressions
- [x] 4.1 Confirm the existing pattern tests (`test_mismatched_lengths_rejected`,
  `test_rest_mismatch_rejected`, `test_song_part_emphasis_mismatch_rejected`)
  still pass alongside the new tests.
