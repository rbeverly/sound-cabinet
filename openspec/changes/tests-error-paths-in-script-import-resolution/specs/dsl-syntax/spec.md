## MODIFIED Requirements

### Requirement: `import <path>` includes another score file

`import <path>` SHALL parse and inline another `.sc` file's contents
at the point of the import. The path SHALL be resolved relative to
the importing file's directory. All declarations
(voice/instrument/fx/wave/patterns/sections), directives, and
statements from the imported file SHALL become part of the
importing file's namespace and timeline. Circular imports SHALL
produce a clear parse error.

When an `import` path cannot be resolved (because the file does not
exist or `canonicalize` fails), `resolve_imports` SHALL return
`Result::Err` whose message identifies the offending path. The
process SHALL NOT panic on a missing import path.

When `resolve_imports` would re-enter a file already on the active
canonicalised-path visited set, it SHALL return `Result::Err` whose
message contains `Circular import detected` and the offending
path. Under no input SHALL it terminate the process with a stack
overflow.

#### Scenario: Import a voice kit
- **WHEN** the score contains `import voices/kit.sc`
- **AND** `voices/kit.sc` defines `instrument piano = ...`
- **THEN** `piano` is available for use in the importing score

#### Scenario: Relative path resolution
- **WHEN** `examples/song.sc` contains `import voices/kit.sc`
- **THEN** the path resolves to `examples/voices/kit.sc`

#### Scenario: Missing import file rejected
- **GIVEN** a script whose only command is
  `import this/does/not/exist.sc`
- **WHEN** `resolve_imports` runs against any base directory that
  does not contain `this/does/not/exist.sc`
- **THEN** the call returns `Err` whose message contains
  `Cannot resolve import` and the unresolved path
- **AND** the process does not panic

#### Scenario: Circular import rejected without panic
- **GIVEN** two files `a.sc` (containing `import b.sc`) and
  `b.sc` (containing `import a.sc`) in the same directory
- **WHEN** a top-level script imports `a.sc`
- **THEN** `resolve_imports` returns `Err` whose message contains
  `Circular import detected` and the canonical path of the file
  that closed the cycle
- **AND** the process does not crash with a stack overflow
