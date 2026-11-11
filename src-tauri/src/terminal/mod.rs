use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

static NEXT_ID: AtomicU32 = AtomicU32::new(1);

pub(crate) struct PtySession {
    writer: Box<dyn Write + Send>,
    master: Box<dyn MasterPty + Send>,
}

pub struct TerminalState(pub Arc<Mutex<HashMap<u32, PtySession>>>);

#[tauri::command]
pub fn pty_create(
    cols: u16,
    rows: u16,
    cwd: String,
    app: AppHandle,
    state: tauri::State<TerminalState>,
) -> Result<u32, String> {
    let pty_system = native_pty_system();

    let pair = pty_system
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| e.to_string())?;

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    let mut cmd = CommandBuilder::new(&shell);
    cmd.cwd(&cwd);

    // Spawn into the slave side
    let _child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
    // Drop slave — child holds it open
    drop(pair.slave);

    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);

    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| e.to_string())?;

    let writer = pair.master.take_writer().map_err(|e| e.to_string())?;

    // Background thread: read PTY output → emit Tauri events
    let app2 = app.clone();
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let data = String::from_utf8_lossy(&buf[..n]).to_string();
                    let _ = app2.emit(&format!("pty_output_{id}"), data);
                }
            }
        }
        let _ = app2.emit(&format!("pty_exit_{id}"), ());
    });

    state
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .insert(
            id,
            PtySession {
                writer,
                master: pair.master,
            },
        );

    Ok(id)
}

#[tauri::command]
pub fn pty_write(
    id: u32,
    data: String,
    state: tauri::State<TerminalState>,
) -> Result<(), String> {
    let mut sessions = state.0.lock().map_err(|e| e.to_string())?;
    if let Some(session) = sessions.get_mut(&id) {
        session
            .writer
            .write_all(data.as_bytes())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn pty_resize(
    id: u32,
    cols: u16,
    rows: u16,
    state: tauri::State<TerminalState>,
) -> Result<(), String> {
    let sessions = state.0.lock().map_err(|e| e.to_string())?;
    if let Some(session) = sessions.get(&id) {
        session
            .master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn pty_kill(id: u32, state: tauri::State<TerminalState>) -> Result<(), String> {
    let mut sessions = state.0.lock().map_err(|e| e.to_string())?;
    sessions.remove(&id);
    Ok(())
}
