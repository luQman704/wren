use std::collections::HashMap;
use std::io::Write;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};

pub struct LspProcess {
    stdin: ChildStdin,
    child: Child,
    last_activity: std::time::Instant,
}

pub struct LspState(pub Arc<Mutex<HashMap<String, LspProcess>>>);

fn write_message(stdin: &mut ChildStdin, content: &str) -> Result<(), String> {
    let bytes = content.as_bytes();
    let header = format!("Content-Length: {}\r\n\r\n", bytes.len());
    stdin.write_all(header.as_bytes()).map_err(|e| e.to_string())?;
    stdin.write_all(bytes).map_err(|e| e.to_string())?;
    stdin.flush().map_err(|e| e.to_string())
}

fn server_command(language: &str) -> Option<(&'static str, Vec<&'static str>)> {
    match language {
        "php" => Some(("intelephense", vec!["--stdio"])),
        "javascript" | "typescript" => Some(("typescript-language-server", vec!["--stdio"])),
        _ => None,
    }
}

fn find_nvm_node_bin() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let nvm_dir = std::path::Path::new(&home).join(".nvm/versions/node");
    let mut versions: Vec<_> = std::fs::read_dir(&nvm_dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('v') { Some((name, e.path())) } else { None }
        })
        .collect();
    // Sort descending to get the latest installed version
    versions.sort_by(|a, b| b.0.cmp(&a.0));
    let (_, path) = versions.into_iter().next()?;
    let bin = path.join("bin");
    if bin.exists() { Some(bin.to_string_lossy().to_string()) } else { None }
}

fn augmented_path() -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let current = std::env::var("PATH").unwrap_or_default();
    let mut extra: Vec<String> = vec![
        "/usr/local/bin".to_string(),
        "/opt/homebrew/bin".to_string(),
        format!("{}/.composer/vendor/bin", home),
        format!("{}/.config/composer/vendor/bin", home),
    ];
    if let Some(node_bin) = find_nvm_node_bin() {
        extra.insert(0, node_bin);
    }
    extra.push(current);
    extra.join(":")
}

pub fn start_idle_checker(state: Arc<Mutex<HashMap<String, LspProcess>>>) {
    const IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10 * 60);
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
        let mut map = match state.lock() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let idle: Vec<String> = map
            .iter()
            .filter(|(_, p)| p.last_activity.elapsed() > IDLE_TIMEOUT)
            .map(|(lang, _)| lang.clone())
            .collect();
        for lang in idle {
            if let Some(mut proc) = map.remove(&lang) {
                let shutdown = r#"{"jsonrpc":"2.0","id":9999,"method":"shutdown","params":null}"#;
                let _ = write_message(&mut proc.stdin, shutdown);
                let exit_note = r#"{"jsonrpc":"2.0","method":"exit","params":null}"#;
                let _ = write_message(&mut proc.stdin, exit_note);
                let _ = proc.child.kill();
            }
        }
    });
}

#[tauri::command]
pub fn lsp_start(
    language: String,
    root_path: String,
    state: State<LspState>,
    app: AppHandle,
) -> Result<(), String> {
    let mut map = state.0.lock().map_err(|e| e.to_string())?;
    if map.contains_key(&language) {
        return Ok(());
    }

    let (cmd, args) = server_command(&language)
        .ok_or_else(|| format!("No LSP server configured for: {}", language))?;

    let mut child = Command::new(cmd)
        .args(&args)
        .env("PATH", augmented_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to start {cmd}: {e}"))?;

    let mut stdin = child.stdin.take().ok_or("No stdin on LSP process")?;
    let stdout = child.stdout.take().ok_or("No stdout on LSP process")?;

    // Send initialize
    let root_uri = format!("file://{}", root_path);
    let workspace_name = root_path.split('/').last().unwrap_or("workspace").to_string();
    let init_msg = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "processId": std::process::id(),
            "rootUri": root_uri,
            "capabilities": {
                "textDocument": {
                    "publishDiagnostics": {
                        "relatedInformation": true,
                        "versionSupport": false
                    },
                    "hover": {
                        "contentFormat": ["markdown", "plaintext"]
                    },
                    "completion": {
                        "completionItem": {
                            "snippetSupport": true,
                            "documentationFormat": ["markdown", "plaintext"],
                            "resolveSupport": { "properties": ["documentation", "detail"] }
                        },
                        "contextSupport": true
                    },
                    "definition": { "linkSupport": false }
                },
                "workspace": {
                    "workspaceFolders": true
                }
            },
            "workspaceFolders": [{
                "uri": root_uri,
                "name": workspace_name
            }]
        }
    })
    .to_string();

    write_message(&mut stdin, &init_msg)?;

    // Background stdout reader — emits "lsp_message_{language}" events
    let lang_clone = language.clone();
    let app_clone = app.clone();
    std::thread::spawn(move || {
        read_loop(stdout, lang_clone, app_clone);
    });

    map.insert(
        language,
        LspProcess {
            stdin,
            child,
            last_activity: std::time::Instant::now(),
        },
    );
    Ok(())
}

fn read_loop(stdout: impl std::io::Read, language: String, app: AppHandle) {
    use std::io::{BufRead, Read};
    let mut reader = std::io::BufReader::new(stdout);
    let event_name = format!("lsp_message_{}", language);

    loop {
        let mut content_length: Option<usize> = None;

        // Read headers until blank line
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => return,
                Ok(_) => {}
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                break;
            }
            if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
                content_length = rest.trim().parse().ok();
            }
        }

        let len = match content_length {
            Some(l) => l,
            None => continue,
        };

        let mut body = vec![0u8; len];
        if reader.read_exact(&mut body).is_err() {
            return;
        }

        if let Ok(msg) = String::from_utf8(body) {
            let _ = app.emit(&event_name, msg);
        }
    }
}

#[tauri::command]
pub fn lsp_send(
    language: String,
    message: String,
    state: State<LspState>,
) -> Result<(), String> {
    let mut map = state.0.lock().map_err(|e| e.to_string())?;
    let proc = map.get_mut(&language).ok_or("LSP not running")?;
    proc.last_activity = std::time::Instant::now();
    write_message(&mut proc.stdin, &message)
}

#[tauri::command]
pub fn lsp_stop(language: String, state: State<LspState>) -> Result<(), String> {
    let mut map = state.0.lock().map_err(|e| e.to_string())?;
    if let Some(mut proc) = map.remove(&language) {
        let shutdown = r#"{"jsonrpc":"2.0","id":9999,"method":"shutdown","params":null}"#;
        let _ = write_message(&mut proc.stdin, shutdown);
        let exit_note = r#"{"jsonrpc":"2.0","method":"exit","params":null}"#;
        let _ = write_message(&mut proc.stdin, exit_note);
        let _ = proc.child.kill();
    }
    Ok(())
}
