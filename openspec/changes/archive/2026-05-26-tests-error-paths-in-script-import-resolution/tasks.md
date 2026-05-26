## 1. Add happy-path tests for `resolve_imports`

- [x] 1.1 `resolve_imports_passes_through_non_import_commands` —
  build a `Script` whose `commands` is `vec![Command::SetBpm { bpm:
  120.0, at_beat: None }]`, call `resolve_imports(script,
  &PathBuf::from("."))`, assert the returned `Script` contains the
  same single `SetBpm` command. Locks in the `other =>` branch
  at `src/dsl/import.rs:46`.
- [x] 1.2 `resolve_imports_inlines_imported_file_contents` —
  create a tempdir with `child.sc` containing `bpm 100`; build an
  in-memory `Script` whose only command is `Command::Import {
  path: "child.sc".into() }`; call `resolve_imports` with the
  tempdir as `base_dir`; assert the result contains exactly one
  `Command::SetBpm { bpm: 100.0, .. }` and zero `Command::Import`
  entries.
- [x] 1.3 `resolve_imports_preserves_surrounding_command_order` —
  in a tempdir, write `mid.sc` with `bpm 90`; build a script
  whose `commands` is `[SetBpm(60), Import("mid.sc"),
  SetBpm(120)]`; assert the returned script's commands, in order,
  are `[SetBpm(60), SetBpm(90), SetBpm(120)]`.
- [x] 1.4 `resolve_imports_handles_nested_imports_relative_to_importer`
  — in a tempdir, create `sub/a.sc` that contains
  `import b.sc`, and `sub/b.sc` that contains `bpm 77`; build a
  script with `Import("sub/a.sc")`; call `resolve_imports` with
  the tempdir as `base_dir`; assert the returned script contains
  `SetBpm(77)`. This locks in the
  `canonical.parent().unwrap_or(base_dir)` behavior at line 42.

## 2. Add error-path tests for `resolve_imports`

- [x] 2.1 `resolve_imports_errors_on_missing_file` — call
  `resolve_imports` on a script importing
  `"this/does/not/exist.sc"` against an empty tempdir; assert
  `Err` whose message contains `"Cannot resolve import"` and the
  path string.
- [x] 2.2 `resolve_imports_errors_on_circular_import` — in a
  tempdir, create `a.sc` containing `import b.sc` and `b.sc`
  containing `import a.sc`; build a script with
  `Import("a.sc")`; assert `Err` whose message contains
  `"Circular import detected"` and the canonical path of `a.sc`
  (since `a` is the entry on the visited set when `b` re-imports
  it).

## 3. Spec update

- [x] 3.1 In `openspec/specs/dsl-syntax/spec.md`, under the
  existing requirement that governs `import` resolution, add a
  `#### Scenario: Missing import file rejected` and a
  `#### Scenario: Circular import rejected without panic`
  matching tests 2.1 and 2.2 — see `specs/dsl-syntax/spec.md` in
  this change directory for the exact wording.
