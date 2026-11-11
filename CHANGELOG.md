# Changelog

## 0.2.0 — 2026-08-25

### Fixed
- **Auto-save no longer triggers "file changed on disk" toast** — own saves are
  now tracked internally and suppressed from the external-change watcher for 3 s.
- **File tree is now vertically scrollable** — sidebar body was missing
  `min-height: 0`, which prevented the flex child from shrinking and scrolling.
- **LSP completions now work** — root cause was a duplicate `autocompletion()`
  StateField: `basicSetup` already registers one, and we were adding a second.
  Replaced `basicSetup` with a hand-rolled equivalent (`makeBaseSetup`) that
  omits autocompletion, then added a single `autocompletion()` instance with
  the LSP + snippet sources correctly in place.
- Fixed misplaced `highlightSelectionMatches` import (moved from
  `@codemirror/autocomplete` to `@codemirror/search` where it actually lives).

### Added
- **Two additional themes** — GitHub Light (light) and Tokyo Night (modern dark),
  alongside the existing One Dark default. Switch between them in
  Preferences → Editor → Theme; selection persists across sessions.
- Theme selector exposed in the Preferences panel with one-click buttons.
- `theme` field added to the persisted prefs file (`~/.config/wren/prefs.json`).

---

## 0.1.0 — 2026-08-24

Initial release.

- Lazy file tree with git status badges and file operations (create / rename / delete)
- Tabbed CodeMirror 6 editor — PHP, JS/TS/JSX/TSX, HTML, Twig, SQL, JSON, YAML,
  Markdown, Python; Drupal file types (`.module`, `.theme`, `.install`, `.inc`,
  `.engine`) mapped to PHP + LSP
- LSP integration: hover, go-to-definition, diagnostics, completions, signature
  help, rename (F2), code actions / lightbulb — Intelephense (PHP) +
  typescript-language-server (JS/TS)
- Breadcrumbs bar showing current class / method from LSP document symbols
- Snippet expansion via `~/.config/wren/snippets/{lang}.json` with Tab-stop nav
- Fuzzy file open (Cmd+P), in-project ripgrep search (Cmd+Shift+F)
- Session persistence per project (`~/.config/wren/sessions/`)
- Preferences panel (Cmd+,) — auto-save, format-on-save, font size,
  Intelephense license key; persisted to `~/.config/wren/prefs.json`
- Format on save: php-cs-fixer (PHP) / prettier (JS/TS)
- Auto-save on by default (1.5 s debounce, configurable)
- Resizable sidebar (160–500 px drag handle)
- Recent projects list
- LSP idle timeout — servers auto-killed after 10 min idle
- LSP error status indicator in status bar (green ● active, red ⚠ failed)
- Lezer syntax error linting (instant red underlines, no LSP required)
- Lint gutter (error / warning markers alongside line numbers)
- CM6 polish: closeBrackets, bracketMatching, highlightActiveLine,
  highlightSelectionMatches, indentation guides, Cmd+F find/replace, Cmd+G go-to-line
- Git diff gutter (green / yellow / red bars for add / modify / delete since HEAD)
- File watcher: toast when open file changed externally
- Integrated terminal (Ctrl+\`) — real PTY, spawns $SHELL, resizable panel
- Window state persistence (size + position remembered across launches)
- Native macOS menu bar (Wren / File / Edit / View)
- Custom wren bird icon (dark navy + blue/green palette)
