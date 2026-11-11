use serde::Serialize;
use std::path::Path;

// Directories that are always hidden from the file tree (CLAUDE.md requirement)
const IGNORED_DIRS: &[&str] = &[
    ".git", "node_modules", "vendor", ".cache", "dist", "build",
    "target", ".svelte-kit", "__pycache__", ".next", ".nuxt",
    "storage", ".idea", ".vscode",
];

#[derive(Debug, Serialize, Clone)]
pub struct FsEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
}

fn is_ignored_dir(name: &str) -> bool {
    IGNORED_DIRS.contains(&name)
}

#[tauri::command]
pub fn read_dir(path: String) -> Result<Vec<FsEntry>, String> {
    let dir = Path::new(&path);

    let mut entries: Vec<FsEntry> = std::fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.file_type().ok()?.is_dir();

            // Skip hidden entries (dotfiles)
            if name.starts_with('.') {
                return None;
            }
            // Skip ignored directories
            if is_dir && is_ignored_dir(&name) {
                return None;
            }

            Some(FsEntry {
                name,
                path: entry.path().to_string_lossy().to_string(),
                is_dir,
            })
        })
        .collect();

    // Directories first, then files — both sorted case-insensitively
    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    Ok(entries)
}

#[tauri::command]
pub fn read_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn write_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

/// Recursively collect all file paths under `root`, respecting the ignore list.
/// Returns paths relative to `root`.
#[tauri::command]
pub fn list_files(root: String) -> Result<Vec<String>, String> {
    let root_path = Path::new(&root);
    let mut results = Vec::new();
    walk(root_path, root_path, &mut results, 0);
    results.sort();
    Ok(results)
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<String>, depth: u32) {
    if depth > 12 {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        if is_dir {
            if is_ignored_dir(&name) {
                continue;
            }
            walk(root, &entry.path(), out, depth + 1);
        } else {
            if let Ok(rel) = entry.path().strip_prefix(root) {
                out.push(rel.to_string_lossy().to_string());
            }
        }
    }
}
