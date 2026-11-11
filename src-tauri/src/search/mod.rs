use serde::{Deserialize, Serialize};
use std::io::Write;
use std::process::Command;

// ── Preferences ───────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Prefs {
    pub auto_save: bool,
    pub auto_save_delay_ms: u64,
    pub format_on_save: bool,
    pub sidebar_width: u32,
    pub intelephense_license_key: Option<String>,
    pub font_size: u32,
    pub theme: String,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            auto_save: true,
            auto_save_delay_ms: 1500,
            format_on_save: false,
            sidebar_width: 220,
            intelephense_license_key: None,
            font_size: 13,
            theme: "one-dark".to_string(),
        }
    }
}

fn prefs_path() -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(std::path::Path::new(&home).join(".config").join("wren").join("prefs.json"))
}

#[tauri::command]
pub fn load_prefs() -> Prefs {
    let path = match prefs_path() { Some(p) => p, None => return Prefs::default() };
    let raw = match std::fs::read_to_string(&path) { Ok(r) => r, Err(_) => return Prefs::default() };
    serde_json::from_str(&raw).unwrap_or_default()
}

#[tauri::command]
pub fn save_prefs(prefs: Prefs) -> Result<(), String> {
    let path = prefs_path().ok_or("no home")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let data = serde_json::to_string_pretty(&prefs).map_err(|e| e.to_string())?;
    std::fs::write(&path, data).map_err(|e| e.to_string())
}

// ── Git diff ──────────────────────────────────────────────────────

#[derive(Debug, Serialize, Clone)]
pub struct DiffLine {
    pub line: usize, // 1-based line number in the new file
    pub kind: String, // "add", "modify", "delete"
}

#[tauri::command]
pub fn git_diff(root: String, path: String) -> Vec<DiffLine> {
    // Compute the path relative to root for git
    let rel_path = if path.starts_with(&root) {
        path.trim_start_matches(&root as &str)
            .trim_start_matches('/')
            .to_string()
    } else {
        path.clone()
    };

    // Check if file is untracked
    let untracked = Command::new("git")
        .arg("-C")
        .arg(&root)
        .arg("ls-files")
        .arg("--others")
        .arg("--exclude-standard")
        .arg("--")
        .arg(&rel_path)
        .output();

    if let Ok(out) = untracked {
        if !out.stdout.is_empty() {
            // File is untracked — count lines and mark all as "add"
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => return vec![],
            };
            return content
                .lines()
                .enumerate()
                .map(|(i, _)| DiffLine {
                    line: i + 1,
                    kind: "add".to_string(),
                })
                .collect();
        }
    }

    // Run git diff HEAD
    let output = Command::new("git")
        .arg("-C")
        .arg(&root)
        .arg("diff")
        .arg("HEAD")
        .arg("--")
        .arg(&rel_path)
        .output();

    let output = match output {
        Ok(o) if o.status.success() || !o.stdout.is_empty() => o,
        _ => return vec![],
    };

    let diff_text = String::from_utf8_lossy(&output.stdout);
    parse_unified_diff(&diff_text)
}

/// Parse a unified diff and return changed line annotations for the new file.
fn parse_unified_diff(diff: &str) -> Vec<DiffLine> {
    let mut results = Vec::new();
    // current line number in the new file
    let mut new_line: usize = 0;
    // lines that were deleted at this position (for "delete" markers)
    let mut pending_deletes: usize = 0;
    // lines that were added contiguously (for "modify" vs "add" distinction)
    let mut contiguous_adds: Vec<usize> = Vec::new();
    let mut in_hunk = false;

    let flush_adds = |adds: &mut Vec<usize>,
                      pending_del: &mut usize,
                      results: &mut Vec<DiffLine>| {
        if adds.is_empty() {
            if *pending_del > 0 {
                // Pure deletion — mark with a delete marker at the line before
                // (we can't show it precisely; use the position after)
                // We'll handle this after flush
            }
        } else if *pending_del > 0 {
            // Mixed: adds + deletes → "modify"
            for &ln in adds.iter() {
                results.push(DiffLine {
                    line: ln,
                    kind: "modify".to_string(),
                });
            }
        } else {
            // Pure addition
            for &ln in adds.iter() {
                results.push(DiffLine {
                    line: ln,
                    kind: "add".to_string(),
                });
            }
        }
        adds.clear();
        *pending_del = 0;
    };

    for raw_line in diff.lines() {
        if raw_line.starts_with("@@") {
            // Flush previous hunk
            flush_adds(&mut contiguous_adds, &mut pending_deletes, &mut results);
            in_hunk = true;

            // Parse @@ -old_start,old_count +new_start,new_count @@
            // e.g. "@@ -10,6 +10,8 @@"
            let parts: Vec<&str> = raw_line.split_whitespace().collect();
            // parts[2] should be "+new_start[,new_count]"
            if parts.len() >= 3 {
                let new_info = parts[2].trim_start_matches('+');
                let start: usize = new_info
                    .split(',')
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1);
                // We start at start - 1 because we'll increment before use
                new_line = start.saturating_sub(1);
            }
            continue;
        }

        if !in_hunk {
            continue;
        }

        if raw_line.starts_with("+++") || raw_line.starts_with("---") {
            continue;
        }

        if raw_line.starts_with('+') {
            new_line += 1;
            contiguous_adds.push(new_line);
        } else if raw_line.starts_with('-') {
            pending_deletes += 1;
        } else {
            // Context line — flush accumulated changes
            flush_adds(&mut contiguous_adds, &mut pending_deletes, &mut results);
            new_line += 1;
        }
    }

    // Flush final hunk
    flush_adds(&mut contiguous_adds, &mut pending_deletes, &mut results);

    results
}

#[derive(Debug, Serialize, Clone)]
pub struct SearchResult {
    pub path: String,      // relative to project root
    pub line_number: u64,
    pub line: String,
    pub match_start: usize,
    pub match_end: usize,
}

#[derive(Debug, Serialize, Clone)]
pub struct GitEntry {
    pub path: String,
    pub status: String, // "M", "A", "D", "?", "R"
}

#[tauri::command]
pub fn search_project(
    root: String,
    query: String,
    case_sensitive: bool,
) -> Result<Vec<SearchResult>, String> {
    if query.trim().is_empty() {
        return Ok(vec![]);
    }

    let mut cmd = Command::new("rg");
    cmd.arg("--json")
        .arg("--max-count=30")
        .arg("--max-filesize=1M")
        .arg("--glob=!vendor")
        .arg("--glob=!node_modules")
        .arg("--glob=!.git")
        .arg("--glob=!target")
        .arg("--glob=!dist")
        .arg("--glob=!build");

    if !case_sensitive {
        cmd.arg("--ignore-case");
    }

    cmd.arg(&query).arg(&root);

    let output = cmd.output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "ripgrep (rg) not found — install with: brew install ripgrep".to_string()
        } else {
            e.to_string()
        }
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut results = Vec::new();

    for line in stdout.lines() {
        let Ok(val) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if val["type"].as_str() != Some("match") {
            continue;
        }
        let data = &val["data"];
        let path = data["path"]["text"].as_str().unwrap_or("").to_string();
        let line_number = data["line_number"].as_u64().unwrap_or(0);
        let line_text = data["lines"]["text"]
            .as_str()
            .unwrap_or("")
            .trim_end_matches(['\n', '\r'])
            .to_string();
        let match_start = data["submatches"][0]["start"].as_u64().unwrap_or(0) as usize;
        let match_end = data["submatches"][0]["end"].as_u64().unwrap_or(0) as usize;

        let rel_path = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .trim_start_matches('/')
            .to_string();

        results.push(SearchResult {
            path: rel_path,
            line_number,
            line: line_text,
            match_start,
            match_end,
        });

        if results.len() >= 500 {
            break;
        }
    }

    Ok(results)
}

#[tauri::command]
pub fn git_status(root: String) -> Result<Vec<GitEntry>, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(&root)
        .arg("status")
        .arg("--porcelain")
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Ok(vec![]); // Not a git repo or git unavailable
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut entries = Vec::new();

    for line in stdout.lines() {
        if line.len() < 3 {
            continue;
        }
        let xy = &line[..2];
        let raw_path = line[3..].trim();
        // Renames show as "old -> new"; take the destination
        let path = raw_path.split(" -> ").last().unwrap_or(raw_path).to_string();

        let status = if xy.contains('M') {
            "M"
        } else if xy.contains('A') {
            "A"
        } else if xy.contains('D') {
            "D"
        } else if xy.contains('R') {
            "R"
        } else if xy.contains('?') {
            "?"
        } else {
            continue;
        };

        entries.push(GitEntry {
            path,
            status: status.to_string(),
        });
    }

    Ok(entries)
}

fn session_path(root: &str) -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").ok()?;
    // FNV-1a hash for a stable filename
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in root.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    Some(
        std::path::Path::new(&home)
            .join(".config")
            .join("wren")
            .join("sessions")
            .join(format!("{:x}.json", hash)),
    )
}

// ── Snippets ──────────────────────────────────────────────────────

fn snippets_path(language: &str) -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(
        std::path::Path::new(&home)
            .join(".config")
            .join("wren")
            .join("snippets")
            .join(format!("{}.json", language)),
    )
}

fn default_snippets(language: &str) -> String {
    match language {
        "php" => PHP_SNIPPETS.to_string(),
        "javascript" | "typescript" => JS_SNIPPETS.to_string(),
        _ => "[]".to_string(),
    }
}

#[tauri::command]
pub fn load_snippets(language: String) -> Result<String, String> {
    let path = snippets_path(&language).ok_or("Could not determine snippets path")?;
    if !path.exists() {
        let defaults = default_snippets(&language);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&path, &defaults).ok();
        return Ok(defaults);
    }
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_snippets(language: String, data: String) -> Result<(), String> {
    let path = snippets_path(&language).ok_or("Could not determine snippets path")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, data).map_err(|e| e.to_string())
}

// Default snippet libraries — stored as JSON so users can edit them directly.
// Body syntax: ${1:placeholder} for tab stops, $0 for final cursor position.

const PHP_SNIPPETS: &str = r#"[
  {
    "prefix": "if",
    "label": "if / elseif / else",
    "description": "if block with optional branches",
    "body": "if (${1:condition}) {\n\t${2}\n}"
  },
  {
    "prefix": "ife",
    "label": "if / else",
    "description": "if-else block",
    "body": "if (${1:condition}) {\n\t${2}\n} else {\n\t${3}\n}"
  },
  {
    "prefix": "fore",
    "label": "foreach",
    "description": "foreach loop",
    "body": "foreach (${1:\\$items} as ${2:\\$item}) {\n\t${3}\n}"
  },
  {
    "prefix": "forek",
    "label": "foreach with key",
    "description": "foreach loop with key => value",
    "body": "foreach (${1:\\$items} as ${2:\\$key} => ${3:\\$value}) {\n\t${4}\n}"
  },
  {
    "prefix": "for",
    "label": "for loop",
    "description": "for loop with counter",
    "body": "for (${1:\\$i} = 0; ${1:\\$i} < ${2:count}; ${1:\\$i}++) {\n\t${3}\n}"
  },
  {
    "prefix": "sw",
    "label": "switch / case",
    "description": "switch statement",
    "body": "switch (${1:\\$value}) {\n\tcase ${2:1}:\n\t\t${3}\n\t\tbreak;\n\n\tdefault:\n\t\t${4}\n\t\tbreak;\n}"
  },
  {
    "prefix": "try",
    "label": "try / catch",
    "description": "try-catch block",
    "body": "try {\n\t${1}\n} catch (${2:\\Exception} ${3:\\$e}) {\n\t${4}\n}"
  },
  {
    "prefix": "trycf",
    "label": "try / catch / finally",
    "description": "try-catch-finally block",
    "body": "try {\n\t${1}\n} catch (${2:\\Exception} ${3:\\$e}) {\n\t${4}\n} finally {\n\t${5}\n}"
  },
  {
    "prefix": "fn",
    "label": "function",
    "description": "function definition",
    "body": "function ${1:name}(${2}): ${3:void} {\n\t${4}\n}"
  },
  {
    "prefix": "cl",
    "label": "class",
    "description": "class skeleton",
    "body": "class ${1:ClassName} {\n\n\tpublic function __construct(\n\t\t${2}\n\t) {}\n\n\t${3}\n}"
  },
  {
    "prefix": "doc",
    "label": "PHPDoc block",
    "description": "PHPDoc comment block",
    "body": "/**\n * ${1:Summary.}\n *\n * @param ${2:type} ${3:\\$param}\n *   ${4:Description.}\n *\n * @return ${5:void}\n */"
  },
  {
    "prefix": "hook",
    "label": "Drupal hook",
    "description": "Drupal hook implementation stub",
    "body": "/**\n * Implements hook_${1:hook_name}().\n */\nfunction ${2:MYMODULE}_${1:hook_name}(${3}) {\n\t${4}\n}"
  },
  {
    "prefix": "eq",
    "label": "Drupal entity query",
    "description": "Drupal entity query pattern",
    "body": "\\$ids = \\Drupal::entityQuery('${1:node}')\n\t->condition('${2:type}', '${3:bundle}')\n\t->condition('status', 1)\n\t->accessCheck(TRUE)\n\t->execute();\n\\$entities = \\Drupal\\${4:node}\\Entity\\${5:Node}::loadMultiple(\\$ids);"
  },
  {
    "prefix": "fa",
    "label": "Drupal form_alter",
    "description": "hook_form_alter implementation",
    "body": "/**\n * Implements hook_form_alter().\n */\nfunction ${1:MYMODULE}_form_alter(array &\\$form, \\Drupal\\Core\\Form\\FormStateInterface \\$form_state, \\$form_id): void {\n\t${2}\n}"
  },
  {
    "prefix": "ffa",
    "label": "Drupal form_FORM_ID_alter",
    "description": "hook_form_FORM_ID_alter for a specific form",
    "body": "/**\n * Implements hook_form_${1:FORM_ID}_alter().\n */\nfunction ${2:MYMODULE}_form_${1:FORM_ID}_alter(array &\\$form, \\Drupal\\Core\\Form\\FormStateInterface \\$form_state): void {\n\t${3}\n}"
  },
  {
    "prefix": "svc",
    "label": "Drupal service class",
    "description": "Drupal service with constructor injection",
    "body": "<?php\n\nnamespace Drupal\\${1:mymodule}\\Service;\n\nuse Drupal\\Core\\Entity\\EntityTypeManagerInterface;\n\nclass ${2:MyService} {\n\n\tpublic function __construct(\n\t\tprivate readonly EntityTypeManagerInterface \\$entityTypeManager,\n\t) {}\n\n\t${3}\n}"
  }
]"#;

const JS_SNIPPETS: &str = r#"[
  {
    "prefix": "fn",
    "label": "arrow function",
    "description": "Arrow function expression",
    "body": "const ${1:name} = (${2:params}) => {\n\t${3}\n};"
  },
  {
    "prefix": "afn",
    "label": "async arrow function",
    "description": "Async arrow function expression",
    "body": "const ${1:name} = async (${2:params}) => {\n\t${3}\n};"
  },
  {
    "prefix": "try",
    "label": "try / catch",
    "description": "try-catch block",
    "body": "try {\n\t${1}\n} catch (${2:err}) {\n\t${3}\n}"
  },
  {
    "prefix": "fore",
    "label": "for...of",
    "description": "for...of loop",
    "body": "for (const ${1:item} of ${2:items}) {\n\t${3}\n}"
  },
  {
    "prefix": "cl",
    "label": "class",
    "description": "Class definition",
    "body": "class ${1:ClassName} {\n\tconstructor(${2}) {\n\t\t${3}\n\t}\n}"
  }
]"#;

// ── Session ───────────────────────────────────────────────────────

#[tauri::command]
pub fn load_session(root: String) -> Option<String> {
    std::fs::read_to_string(session_path(&root)?).ok()
}

#[tauri::command]
pub fn save_session(root: String, data: String) -> Result<(), String> {
    let path = session_path(&root).ok_or("Could not determine session path")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, data).map_err(|e| e.to_string())
}

// ── File operations ───────────────────────────────────────────────

#[tauri::command]
pub fn create_file(path: String) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::File::create(&path).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn create_dir(path: String) -> Result<(), String> {
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rename_path(old_path: String, new_path: String) -> Result<(), String> {
    std::fs::rename(&old_path, &new_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_path(path: String) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    if p.is_dir() {
        std::fs::remove_dir_all(&path).map_err(|e| e.to_string())
    } else {
        std::fs::remove_file(&path).map_err(|e| e.to_string())
    }
}

// ── Format on save ────────────────────────────────────────────────

#[tauri::command]
pub fn format_file(path: String, content: String, language: String) -> Result<String, String> {
    match language.as_str() {
        "php" => format_php(&path, &content),
        "javascript" | "typescript" => format_js(&path, &content),
        _ => Ok(content),
    }
}

fn format_php(_path: &str, content: &str) -> Result<String, String> {
    let tmp = std::env::temp_dir().join(format!(
        "wren_fmt_{}.php",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos()
    ));
    std::fs::write(&tmp, content).map_err(|e| e.to_string())?;
    let output = Command::new("php-cs-fixer")
        .arg("fix")
        .arg("--quiet")
        .arg("--rules=@PSR12")
        .arg(tmp.to_str().unwrap_or(""))
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "php-cs-fixer not found — install with: composer global require friendsofphp/php-cs-fixer".to_string()
            } else {
                e.to_string()
            }
        })?;
    if !output.status.success() {
        let _ = std::fs::remove_file(&tmp);
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("php-cs-fixer failed: {}", stderr));
    }
    let result = std::fs::read_to_string(&tmp).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&tmp);
    Ok(result)
}

fn format_js(path: &str, content: &str) -> Result<String, String> {
    let filename = std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file.js");
    let mut child = Command::new("prettier")
        .arg("--stdin-filepath")
        .arg(filename)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "prettier not found — install with: npm install -g prettier".to_string()
            } else {
                e.to_string()
            }
        })?;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(content.as_bytes())
        .map_err(|e| e.to_string())?;
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("prettier failed: {}", stderr));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

// ── Recent projects ───────────────────────────────────────────────

fn recent_path() -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(
        std::path::Path::new(&home)
            .join(".config")
            .join("wren")
            .join("recent.json"),
    )
}

#[tauri::command]
pub fn load_recent() -> Vec<String> {
    let path = match recent_path() {
        Some(p) => p,
        None => return vec![],
    };
    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

#[tauri::command]
pub fn save_recent(paths: Vec<String>) -> Result<(), String> {
    let path = recent_path().ok_or("No home")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut deduped: Vec<String> = vec![];
    for p in paths {
        if !deduped.contains(&p) {
            deduped.push(p);
        }
        if deduped.len() >= 10 {
            break;
        }
    }
    std::fs::write(path, serde_json::to_string(&deduped).unwrap()).map_err(|e| e.to_string())
}
