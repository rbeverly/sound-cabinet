use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};

use super::ast::{Command, Script};
use super::parser::parse_script;

/// Resolve all `import` commands by inlining the contents of imported files.
/// Paths are resolved relative to `base_dir`. Circular imports are detected.
pub fn resolve_imports(script: Script, base_dir: &Path) -> Result<Script> {
    let mut visited = HashSet::new();
    resolve_recursive(script, base_dir, &mut visited)
}

fn resolve_recursive(
    script: Script,
    base_dir: &Path,
    visited: &mut HashSet<PathBuf>,
) -> Result<Script> {
    let mut output = Vec::new();

    for cmd in script.commands {
        match cmd {
            Command::Import { ref path } => {
                let full_path = base_dir.join(path);
                let canonical = full_path
                    .canonicalize()
                    .map_err(|e| anyhow!("Cannot resolve import '{}': {e}", path))?;

                if !visited.insert(canonical.clone()) {
                    return Err(anyhow!(
                        "Circular import detected: {}",
                        canonical.display()
                    ));
                }

                let source = std::fs::read_to_string(&canonical)
                    .map_err(|e| anyhow!("Cannot read '{}': {e}", canonical.display()))?;

                let imported = parse_script(&source)?;
                let import_dir = canonical.parent().unwrap_or(base_dir);
                let resolved = resolve_recursive(imported, import_dir, visited)?;
                output.extend(resolved.commands);
            }
            other => output.push(other),
        }
    }

    Ok(Script { commands: output })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    /// RAII guard that removes its tempdir on drop, even if a test panics.
    struct TempDirGuard(PathBuf);

    impl TempDirGuard {
        fn new(label: &str) -> Self {
            let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
            let pid = std::process::id();
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let dir = std::env::temp_dir()
                .join(format!("sc-import-test-{label}-{pid}-{nanos}-{n}"));
            fs::create_dir_all(&dir).expect("create tempdir");
            TempDirGuard(dir)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDirGuard {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn write_file(dir: &Path, name: &str, contents: &str) {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent dir");
        }
        fs::write(path, contents).expect("write file");
    }

    fn import_cmd(path: &str) -> Command {
        Command::Import { path: path.to_string() }
    }

    fn set_bpm(bpm: f64) -> Command {
        Command::SetBpm { bpm, at_beat: None }
    }

    #[test]
    fn resolve_imports_passes_through_non_import_commands() {
        let script = Script { commands: vec![set_bpm(120.0)] };
        let result = resolve_imports(script, &PathBuf::from(".")).expect("resolve");
        assert_eq!(result.commands.len(), 1);
        match &result.commands[0] {
            Command::SetBpm { bpm, at_beat } => {
                assert_eq!(*bpm, 120.0);
                assert!(at_beat.is_none());
            }
            other => panic!("expected SetBpm, got {other:?}"),
        }
    }

    #[test]
    fn resolve_imports_inlines_imported_file_contents() {
        let tmp = TempDirGuard::new("inline");
        write_file(tmp.path(), "child.sc", "bpm 100\n");

        let script = Script { commands: vec![import_cmd("child.sc")] };
        let result = resolve_imports(script, tmp.path()).expect("resolve");

        assert_eq!(result.commands.len(), 1, "expected exactly one command");
        match &result.commands[0] {
            Command::SetBpm { bpm, .. } => assert_eq!(*bpm, 100.0),
            other => panic!("expected SetBpm(100), got {other:?}"),
        }
        let import_count = result
            .commands
            .iter()
            .filter(|c| matches!(c, Command::Import { .. }))
            .count();
        assert_eq!(import_count, 0, "expected zero Import commands after resolve");
    }

    #[test]
    fn resolve_imports_preserves_surrounding_command_order() {
        let tmp = TempDirGuard::new("order");
        write_file(tmp.path(), "mid.sc", "bpm 90\n");

        let script = Script {
            commands: vec![set_bpm(60.0), import_cmd("mid.sc"), set_bpm(120.0)],
        };
        let result = resolve_imports(script, tmp.path()).expect("resolve");

        let bpms: Vec<f64> = result
            .commands
            .iter()
            .map(|c| match c {
                Command::SetBpm { bpm, .. } => *bpm,
                other => panic!("unexpected non-SetBpm command: {other:?}"),
            })
            .collect();
        assert_eq!(bpms, vec![60.0, 90.0, 120.0]);
    }

    #[test]
    fn resolve_imports_handles_nested_imports_relative_to_importer() {
        let tmp = TempDirGuard::new("nested");
        write_file(tmp.path(), "sub/a.sc", "import b.sc\n");
        write_file(tmp.path(), "sub/b.sc", "bpm 77\n");

        let script = Script { commands: vec![import_cmd("sub/a.sc")] };
        let result = resolve_imports(script, tmp.path()).expect("resolve");

        assert_eq!(result.commands.len(), 1);
        match &result.commands[0] {
            Command::SetBpm { bpm, .. } => assert_eq!(*bpm, 77.0),
            other => panic!("expected SetBpm(77), got {other:?}"),
        }
    }

    #[test]
    fn resolve_imports_errors_on_missing_file() {
        let tmp = TempDirGuard::new("missing");
        let missing = "this/does/not/exist.sc";

        let script = Script { commands: vec![import_cmd(missing)] };
        let err = resolve_imports(script, tmp.path()).expect_err("expected Err");
        let msg = format!("{err}");

        assert!(
            msg.contains("Cannot resolve import"),
            "expected message to contain 'Cannot resolve import', got: {msg}"
        );
        assert!(
            msg.contains(missing),
            "expected message to contain the missing path '{missing}', got: {msg}"
        );
    }

    #[test]
    fn resolve_imports_errors_on_circular_import() {
        let tmp = TempDirGuard::new("circular");
        write_file(tmp.path(), "a.sc", "import b.sc\n");
        write_file(tmp.path(), "b.sc", "import a.sc\n");

        let canonical_a = tmp.path().join("a.sc").canonicalize().expect("canonicalize a.sc");

        let script = Script { commands: vec![import_cmd("a.sc")] };
        let err = resolve_imports(script, tmp.path()).expect_err("expected Err");
        let msg = format!("{err}");

        assert!(
            msg.contains("Circular import detected"),
            "expected message to contain 'Circular import detected', got: {msg}"
        );
        assert!(
            msg.contains(&canonical_a.display().to_string()),
            "expected message to contain canonical path of a.sc ({}), got: {msg}",
            canonical_a.display()
        );
    }
}
