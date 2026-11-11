mod fs;
mod lsp;
mod search;
mod terminal;
mod watcher;

use lsp::LspState;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::menu::{MenuBuilder, SubmenuBuilder};
use tauri::{Emitter, Manager};
use terminal::TerminalState;
use watcher::WatcherState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let lsp_map = Arc::new(Mutex::new(HashMap::new()));
    let lsp_arc = Arc::clone(&lsp_map);
    lsp::start_idle_checker(lsp_arc);
    let lsp_state = LspState(lsp_map);

    let terminal_state = TerminalState(Arc::new(Mutex::new(HashMap::new())));
    let watcher_state = WatcherState(Mutex::new(None));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .manage(lsp_state)
        .manage(terminal_state)
        .manage(watcher_state)
        .invoke_handler(tauri::generate_handler![
            fs::read_dir,
            fs::read_file,
            fs::write_file,
            fs::list_files,
            lsp::lsp_start,
            lsp::lsp_send,
            lsp::lsp_stop,
            search::search_project,
            search::git_status,
            search::git_diff,
            search::load_session,
            search::save_session,
            search::load_snippets,
            search::save_snippets,
            search::create_file,
            search::create_dir,
            search::rename_path,
            search::delete_path,
            search::format_file,
            search::load_recent,
            search::save_recent,
            search::load_prefs,
            search::save_prefs,
            terminal::pty_create,
            terminal::pty_write,
            terminal::pty_resize,
            terminal::pty_kill,
            watcher::watch_project,
            watcher::unwatch_project,
        ])
        .setup(|app| {
            let wren_menu = SubmenuBuilder::new(app, "Wren")
                .text("about", "About Wren")
                .separator()
                .text("preferences", "Preferences\u{0009}⌘,")
                .separator()
                .text("hide", "Hide Wren")
                .text("quit", "Quit Wren\u{0009}⌘Q")
                .build()?;

            let file_menu = SubmenuBuilder::new(app, "File")
                .text("open-folder", "Open Folder…\u{0009}⌘O")
                .separator()
                .text("close-tab", "Close Tab\u{0009}⌘W")
                .text("save", "Save\u{0009}⌘S")
                .build()?;

            let edit_menu = SubmenuBuilder::new(app, "Edit")
                .text("undo", "Undo\u{0009}⌘Z")
                .text("redo", "Redo\u{0009}⇧⌘Z")
                .separator()
                .cut()
                .copy()
                .paste()
                .select_all()
                .separator()
                .text("find", "Find…\u{0009}⌘F")
                .text("find-replace", "Find and Replace\u{0009}⌥⌘F")
                .build()?;

            let view_menu = SubmenuBuilder::new(app, "View")
                .text("toggle-sidebar", "Toggle Sidebar\u{0009}⌘B")
                .text("toggle-terminal", "Toggle Terminal\u{0009}⌃`")
                .separator()
                .text("command-palette", "Command Palette\u{0009}⌘P")
                .text("search", "Search in Project\u{0009}⇧⌘F")
                .build()?;

            let menu = MenuBuilder::new(app)
                .item(&wren_menu)
                .item(&file_menu)
                .item(&edit_menu)
                .item(&view_menu)
                .build()?;

            app.set_menu(menu)?;

            app.on_menu_event(|app, event| {
                let window = match app.get_webview_window("main") {
                    Some(w) => w,
                    None => return,
                };
                match event.id().as_ref() {
                    "preferences"     => { let _ = window.emit("menu_preferences", ()); }
                    "open-folder"     => { let _ = window.emit("menu_open_folder", ()); }
                    "close-tab"       => { let _ = window.emit("menu_close_tab", ()); }
                    "save"            => { let _ = window.emit("menu_save", ()); }
                    "toggle-sidebar"  => { let _ = window.emit("menu_toggle_sidebar", ()); }
                    "toggle-terminal" => { let _ = window.emit("menu_toggle_terminal", ()); }
                    "command-palette" => { let _ = window.emit("menu_command_palette", ()); }
                    "search"          => { let _ = window.emit("menu_search", ()); }
                    "find"            => { let _ = window.emit("menu_find", ()); }
                    "quit"            => { app.exit(0); }
                    _ => {}
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
