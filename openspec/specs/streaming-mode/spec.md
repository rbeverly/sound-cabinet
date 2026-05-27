# streaming-mode Specification

## Purpose
TBD - created by archiving change initial-spec-baseline. Update Purpose after archive.
## Requirements
### Requirement: `stream` command reads stdin and plays in real time

The `stream` subcommand SHALL take no positional arguments. It SHALL start an audio output stream immediately and read lines from stdin until EOF (Ctrl+D), parsing each non-empty non-comment line as a single `.sc` directive and applying it to the live engine.

#### Scenario: Start streaming mode
- **WHEN** the user runs `sound-cabinet stream`
- **THEN** the program initializes an empty engine at 44.1 kHz and starts audio output
- **AND** stderr contains `Streaming mode. Type score lines, Ctrl+D to end.`
- **AND** the program reads lines from stdin

#### Scenario: Stream ends on EOF
- **WHEN** stdin closes (Ctrl+D or upstream process exits)
- **THEN** the dispatcher emits a shutdown signal
- **AND** the audio thread shuts down cleanly
- **AND** the program exits with status 0

#### Scenario: Pipe input from another process
- **WHEN** another process pipes `.sc` lines into `sound-cabinet stream` (e.g. `echo "bpm 120\nat 0 play sine(A4) for 2 beats" | sound-cabinet stream`)
- **THEN** each line is parsed and applied in order
- **AND** events play through the speakers as they arrive

### Requirement: Empty lines and comments are skipped

The stdin reader SHALL trim whitespace from each line. Lines that are empty after trimming SHALL be ignored. Lines beginning with `//` after trimming SHALL be ignored as comments and not sent to the dispatcher.

#### Scenario: Blank lines ignored
- **WHEN** a blank or whitespace-only line is received
- **THEN** it is silently skipped — no parse error, no engine action

#### Scenario: Comment lines ignored
- **WHEN** a line begins with `//` (after whitespace trim)
- **THEN** it is silently skipped

### Requirement: `at N` is interpreted relative to "now", not beat 0

In streaming mode, the `at <N>` timing on a `play` directive SHALL be interpreted as `N` beats from the engine's current playback position, not from absolute beat 0. `at 0` SHALL mean "play immediately"; `at 1` SHALL mean "play one beat from now".

#### Scenario: `at 0` plays immediately
- **WHEN** the user pipes `at 0 play sine(A4) for 2 beats` into `stream`
- **THEN** the engine schedules the event to start at the current playback position
- **AND** the tone begins playing immediately (within audio buffer latency)

#### Scenario: `at N` plays N beats in the future
- **WHEN** the user pipes `at 4 play sine(C5) for 1 beats` while the engine has been playing for some time
- **THEN** the engine schedules the event to start 4 beats from the current playback position

### Requirement: Streaming supports voice definitions and BPM changes

The streaming dispatcher SHALL recognize and apply single-line directives: voice definitions (`voice name = expr`), BPM changes (`bpm N`), and play events (`at N play <expr> for <M> beats`). Other constructs (patterns, sections, sequence/repeat blocks, `with` substitutions, `master ...`) SHALL be ignored or noop'd because streaming mode handles only single-line commands.

#### Scenario: Define a voice mid-stream
- **WHEN** the user sends the line `voice pad = saw(C3) >> lowpass(800, 0.7)`
- **THEN** the engine registers a new voice named `pad` with the given expression
- **AND** subsequent `play pad` events use the new definition

#### Scenario: Change tempo mid-stream
- **WHEN** the user sends `bpm 140`
- **THEN** the engine's tempo updates to 140 BPM for all subsequently scheduled events

#### Scenario: Multi-line constructs are not supported
- **WHEN** the user sends a line that would normally begin a multi-line `pattern` or `section` block
- **THEN** the dispatcher does NOT enter a multi-line mode
- **AND** no error is raised but the construct is treated as a no-op

### Requirement: Parse errors emit a message and continue

When a stdin line fails to parse, the dispatcher SHALL emit `Parse error: <reason>` to stderr and continue reading subsequent lines without exiting.

#### Scenario: Malformed line continues stream
- **WHEN** the user sends a malformed line such as `at xyz play foo`
- **THEN** stderr contains `Parse error: <reason>`
- **AND** the next stdin line is still parsed and applied
- **AND** the program does NOT exit

