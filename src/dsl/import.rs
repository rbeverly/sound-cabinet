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
