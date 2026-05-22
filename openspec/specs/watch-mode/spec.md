# watch-mode Specification

## Purpose
Run continuous playback of a `.sc` score while watching its containing directory for `.sc` file changes, rebuilding and swapping the engine in real time without dropping the audio stream. Designed for iterative live-coding workflows where you edit the score in an editor and want changes to take effect on save.

## Requirements

### Requirement: `watch` command plays a score and reloads on save

The `watch` subcommand SHALL accept a `.sc` source file, immediately begin real-time playback of its rendered, fully-mastered output, AND watch the file's parent directory recursively for changes to any `.sc` file. When an `.sc` file changes, the engine SHALL be rebuilt from the original score path and atomically swapped without stopping the audio stream.

#### Scenario: Initial playback and watch start
- **WHEN** the user runs `sound-cabinet watch <score.sc>`
- **THEN** the engine builds from `<score.sc>` and begins streaming audio
- **AND** stderr contains `Watching <score.sc> (Ctrl+C to stop)`
- **AND** stderr contains `Playing...`
- **AND** the parent directory of `<score.sc>` is being watched recursively

#### Scenario: Reload after `.sc` file save
- **GIVEN** `watch` is running for `<score.sc>`
- **WHEN** any `.sc` file in the watched directory changes on disk
- **THEN** the watcher waits 200ms (debounce) and drains any additional pending events
- **AND** stderr contains `\nFile changed, rebuilding...`
- **AND** the engine is rebuilt from `<score.sc>` and swapped in atomically
- **AND** stderr contains `Playing...` after the swap

#### Scenario: Non-`.sc` file changes are ignored
- **WHEN** a file change is detected for a path whose extension is not `.sc`
- **THEN** the engine is NOT rebuilt and the audio stream continues unchanged

#### Scenario: Rebuild fails on parse error
- **GIVEN** `watch` is running with a valid engine playing
- **WHEN** an `.sc` file save introduces a syntax or other build error
- **THEN** stderr contains `Error: <reason>` followed by `(keeping previous version)`
- **AND** the previously loaded engine continues playing uninterrupted

### Requirement: Watch supports `-v` / `--verbose` and `--from <beat>`

The `watch` command SHALL accept the same `-v` / `--verbose` flag (printing beat positions and pattern names) and `--from <beat>` flag (skip-to-beat) as `play`. The `--from` setting SHALL be re-applied to every rebuilt engine.

#### Scenario: Verbose watch
- **WHEN** the user runs `watch <score.sc> -v`
- **THEN** each engine instance (initial and after every reload) has verbose mode enabled

#### Scenario: From-beat preserved across reloads
- **WHEN** the user runs `watch <score.sc> --from 32`
- **THEN** the initial engine skips to beat 32
- **AND** every reloaded engine also skips to beat 32

#### Scenario: Missing score path
- **WHEN** the user runs `sound-cabinet watch` with no arguments
- **THEN** the program exits non-zero with a usage message naming `<score.sc>`

### Requirement: Watch exits cleanly on Ctrl+C

The `watch` command SHALL handle Ctrl+C by shutting down the audio thread and the file-watcher cleanly. The program SHALL exit with status 0 on graceful shutdown.

#### Scenario: User stops watch
- **GIVEN** `watch` is running
- **WHEN** the user presses Ctrl+C
- **THEN** the audio stream is stopped
- **AND** the file watcher is shut down
- **AND** the program exits with status 0
