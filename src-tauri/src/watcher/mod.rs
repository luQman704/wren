use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc;
use tauri::{AppHandle, Emitter};

pub struct WatcherState(pub std::sync::Mutex<Option<RecommendedWatcher>>);

/// Directories to exclude from file-change events.
const IGNORE_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "vendor",
    "target",
    "dist",
    "build",
    ".cache",
    "storage/framework",
];

fn should_ignore(path: &std::path::Path) -> bool {
    for component in path.components() {
        let s = component.as_os_str().to_string_lossy();
        if IGNORE_DIRS.iter().any(|&ign| s == ign) {
            return true;
        }
    }
    false
}

#[tauri::command]
pub fn watch_project(
    root: String,
    app: AppHandle,
    state: tauri::State<WatcherState>,
) -> Result<(), String> {
    // Stop any existing watcher first
    {
        let mut guard = state.0.lock().map_err(|e| e.to_string())?;
        *guard = None; // drop the old watcher
    }

    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();

    let mut watcher = RecommendedWatcher::new(
        move |res| {
            let _ = tx.send(res);
        },
        notify::Config::default(),
    )
    .map_err(|e| e.to_string())?;

    let root_path = std::path::PathBuf::from(&root);
    watcher
        .watch(&root_path, RecursiveMode::Recursive)
        .map_err(|e| e.to_string())?;

    // Background thread: forward relevant events as Tauri events
    let app2 = app.clone();
    std::thread::spawn(move || {
        for res in rx {
            let event = match res {
                Ok(e) => e,
                Err(_) => continue,
            };

            // We care about create, modify, remove
            match event.kind {
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {}
                _ => continue,
            }

            for path in event.paths {
                if should_ignore(&path) {
                    continue;
                }
                let path_str = path.to_string_lossy().to_string();
                let _ = app2.emit("file_changed", serde_json::json!({ "path": path_str }));
            }
        }
    });

    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    *guard = Some(watcher);

    Ok(())
}

#[tauri::command]
pub fn unwatch_project(state: tauri::State<WatcherState>) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    *guard = None;
    Ok(())
}
