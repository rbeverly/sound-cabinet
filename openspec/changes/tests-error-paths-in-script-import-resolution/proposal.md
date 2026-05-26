## Why

`src/dsl/import.rs::resolve_imports` is responsible for inlining
`import` commands when a `.sc` file is loaded. The file has
**zero tests** — neither happy paths nor error paths — yet it
contains three distinct `Err`-returning branches that are described
in the `dsl-syntax` capability:

- `src/dsl/import.rs:29` — `Cannot resolve import '<path>': <io>`
  when `Path::canonicalize` fails (file does not exist, dangling
  symlink, etc.).
- `src/dsl/import.rs:32` — `Circular import detected: <path>`
  when an already-visited canonical path is seen again.
- `src/dsl/import.rs:39` — `Cannot read '<path>': <io>` when the
  file exists at `canonicalize` time but cannot be read.

The happy paths are also untested:

- A non-`Import` `Command` flows through unchanged
  (`other => output.push(other)`).
- A successful import inlines the imported script's commands at the
  call site and preserves order with surrounding commands.
- Nested imports resolve relative to the importer's directory
  (`canonical.parent().unwrap_or(base_dir)`).

A regression in any of these paths would silently break every
multi-file `.sc` project shipped under `examples/`. The dsl-syntax
spec specifically requires "Circular imports rejected" but there is
no test wired to that requirement today.

## What Changes

Create a new file `src/dsl/import_tests.rs` (or add a
`#[cfg(test)] mod tests` block at the bottom of
`src/dsl/import.rs` — implementer's choice). Add the following
tests using `tempfile::TempDir` (or `std::env::temp_dir()` plus a
unique subdirectory) to materialise `.sc` files on disk:

- `resolve_imports_passes_through_non_import_commands`
- `resolve_imports_inlines_imported_file_contents`
- `resolve_imports_preserves_surrounding_command_order`
- `resolve_imports_handles_nested_imports_relative_to_importer`
- `resolve_imports_errors_on_missing_file`
- `resolve_imports_errors_on_circular_import`

Also lock the invariant in the `dsl-syntax` spec.

## Impact

- `src/dsl/import.rs` — add a `#[cfg(test)] mod tests` block (or
  a sibling `import_tests.rs` if the implementer prefers
  isolating the filesystem fixtures).
- `Cargo.toml` — add `tempfile = "3"` under `[dev-dependencies]`
  if the implementer chooses `tempfile`. Plain
  `std::env::temp_dir()` with a per-test subdirectory and explicit
  cleanup is also acceptable and adds no new dep.
- `openspec/specs/dsl-syntax/spec.md` — see the `specs/`
  modification below: a new `#### Scenario:` is added under the
  existing "Imports" / circular-import requirement.
- No production code changes. No existing test is modified.
