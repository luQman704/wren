# Project: Wren — a lightweight PHP/JS editor

## Why this project exists

PhpStorm on this machine (2019 MacBook Pro, Intel i9, 32GB RAM, 1TB SSD, macOS
Tahoe) regularly hangs the whole system when OrbStack spins up ddev's Docker
containers alongside it. The suspected cause is PhpStorm's indexer: it walks
and indexes `vendor/`, `node_modules/`, VCS history, and other generated
directories on top of everything Docker is already doing, and the combined
memory/CPU pressure on an aging Intel Mac tips the system over.

The goal of this project is **not** to rebuild PhpStorm. It's to build a
small, fast, single-purpose editor for the languages actually used in this
day-to-day full-stack (Drupal/PHP-heavy) workflow, that:

- Starts in well under a second
- Never indexes or watches `vendor/`, `node_modules/`, `.git/`, build/cache
  dirs by default
- Keeps a low, predictable memory footprint even with large projects open
  alongside ddev/OrbStack
- Is tailored to one person's workflow, not a general-purpose extensible
  platform

If it takes noticeably fewer resources than PhpStorm while covering "edit +
navigate + basic LSP smarts," it's a success — it doesn't need to match
PhpStorm feature-for-feature.

## Target languages / file types

Primary (need real LSP + syntax highlighting):
- PHP
- JavaScript / TypeScript

Secondary (syntax highlighting is enough to start; LSP later if it's cheap):
- HTML
- Twig
- SQL
- YAML / JSON
- Markdown

## Non-goals (explicitly out of scope for now)

- Not a general extension/plugin marketplace
- Not trying to replicate PhpStorm's deep refactoring tools (rename-across-
  project, safe delete, etc.) — basic LSP rename/go-to-definition is enough
- Not building a debugger (Xdebug) integration in the MVP
- Not a multi-language platform beyond the list above
- Not trying to be faster than Vim/Helix at their own game — this is an
  editor with a GUI, tabs, and a file tree, tuned for this one workflow

## Tech stack

- **Shell / runtime:** [Tauri](https://tauri.app) (Rust backend + OS-native
  webview). Chosen over Electron specifically because it doesn't bundle a
  full Chromium + Node runtime per app instance — this matters given the
  memory-pressure problem we're solving for.
- **Frontend:** TypeScript, no heavy framework by default — prefer a small
  reactive layer (e.g. Svelte, or SolidJS) over React if a framework is
  needed at all, to keep the render layer light.
- **Editor component:** [CodeMirror 6](https://codemirror.net/) — not
  Monaco. Monaco is powerful but heavy (it's VS Code's own editor engine);
  CodeMirror 6 is modular, tree-shakeable, and much lighter for a
  single-purpose tool.
- **Syntax highlighting / parsing:** tree-sitter grammars (php, javascript,
  typescript, html, twig, sql) via CodeMirror's tree-sitter integration
  where available, falling back to CodeMirror's built-in language modes
  where a tree-sitter grammar isn't practical.
- **Language servers (reuse, don't reinvent):**
  - PHP: [Intelephense](https://intelephense.com/) or
    [Phpactor](https://phpactor.org/) — evaluate both for memory footprint,
    prefer whichever is lighter in practice
  - JS/TS: `typescript-language-server`
  - Twig: evaluate available Twig LSPs; if none are lightweight, ship
    highlighting-only for Twig initially
  - SQL: optional/deferred — highlighting first, LSP later if a light one
    exists
  - The app is an **LSP client**, spawning existing language servers as
    subprocesses — never reimplement PHP/JS language intelligence in-house
- **Search:** shell out to `ripgrep` rather than building/maintaining a
  custom file index
- **File watching:** OS-native watcher (e.g. `notify` crate on the Rust
  side), scoped and debounced — never a naive full-tree poll

## Code completion (class/method autocomplete)

No AI involved here — this is standard LSP completion, and it's effectively
"free" given the app is already an LSP client. Intelephense/Phpactor
already understand PHP's class hierarchy, autoloading (including Composer's
autoloader, so Drupal/Laravel classes resolve correctly), and namespaces;
`typescript-language-server` does the equivalent for JS/TS. The work here is
wiring up the editor UI on top of the `textDocument/completion` responses
the LSP already provides, not building any completion intelligence
in-house.

What this needs:

- **Trigger characters:** request completions on LSP-standard triggers —
  `->` and `::` in PHP (instance/static member access), `.` in JS/TS, plus
  manual invocation (e.g. Ctrl+Space) anywhere.
- **Completion popup:** a filterable dropdown at the cursor showing
  candidates from the LSP response — class names, methods, properties,
  constants — with fuzzy-filtering as you keep typing.
- **Item detail:** show each candidate's type signature/return type and, if
  the LSP provides it, a short doc-comment/docblock summary
  (`completionItem/resolve` for the extra detail on demand, not eagerly for
  every item in the list — keep the initial popup response fast).
- **Insertion behavior:** support LSP snippet-style insertion where the
  server provides it (e.g. auto-filling required parameters), and commit
  characters (Tab/Enter) to accept.
- **Auto-import on completion (stretch, not MVP):** some PHP LSPs can add
  the correct `use` statement when you accept a completion for a class
  that isn't imported yet — worth wiring up once basic completion works,
  not before.
- Same resource discipline as everything else in this doc: completions
  come from the already-running LSP process, so this doesn't add new
  background work, network calls, or memory pressure beyond what the LSP
  integration already costs.

## Snippets (block templates)

Separate from class/method completion above: **snippet expansion** for
common code blocks — `switch`/`case`, `foreach`, `try`/`catch`,
`if`/`elseif`/`else`, function/class skeletons, PHPDoc blocks, and anything
Drupal/Laravel-specific worth templating (e.g. a hook implementation
skeleton, an entity query pattern, a form-alter stub). Type a short prefix,
accept (Tab/Enter), then Tab through placeholders to fill in the parts that
change (the switch expression, each case value, the exception type, etc.).

Two complementary sources, not an either/or:

- **Editor-native snippets (primary):** define these in the app itself
  using CodeMirror 6's built-in snippet support
  (`@codemirror/autocomplete`'s `snippet()` completion source, which
  already handles `${placeholder}` syntax and Tab-navigation between
  placeholders). This is the source of truth for anything project- or
  workflow-specific, since it's not dependent on what a given LSP happens
  to ship.
- **LSP-provided snippets (supplementary):** where the active language
  server already offers snippet-formatted completions for generic
  constructs (Intelephense does for some PHP control structures), merge
  those into the same completion popup as editor-native ones rather than
  building duplicates by hand.

Practical details:

- **Storage:** keep snippets in a plain, user-editable file (e.g. TOML or
  JSON) per language, not hardcoded in the Rust/frontend source — the
  point of "tailored to me" is being able to add or tweak one without a
  rebuild. A per-project override file (for Drupal-specific vs
  Laravel-specific snippets on different projects) is a reasonable later
  addition, not required for the first version.
- **Trigger:** prefix-based, shown in the same completion popup as
  LSP class/method suggestions rather than a separate UI — the two
  feel like one "completion" feature from the keyboard even though the
  data sources differ.
- **Start small:** ship the handful of PHP constructs above plus a couple
  of real Drupal patterns actually used day-to-day, and add more
  opportunistically rather than trying to pre-build a large library.

## UI direction

No strong aesthetic requirement going in — default to something sensible
and iterate once there's an actual screen to react to: dark theme,
minimal chrome (no toolbars/menus fighting for space with the editor),
monospace throughout, generous use of whitespace over dense panels. Treat
this as a placeholder to revise the first time the real UI feels wrong,
not a spec to defend.

## Memory-conscious design rules (the whole point of this project)

These are hard constraints, not nice-to-haves:

1. **Ignore-list is load-bearing, not cosmetic.** Respect `.gitignore` plus a
   built-in default ignore list (`vendor/`, `node_modules/`, `.git/`,
   `storage/framework/`, `web/sites/*/files/`, build/cache dirs) for file
   tree rendering, file watching, and search — always, unless the user
   explicitly opts a path back in.
2. **No full-project indexing on open.** Don't eagerly walk and parse every
   file in a project at startup. Build the file tree lazily (expand-on-
   demand directories), and let the LSP server handle symbol
   indexing/search on its own terms — don't duplicate that work in the app.
3. **LSP processes are spawned on demand and killed when idle.** Don't run
   every language server for every project at all times; start the PHP
   server only when a PHP file is opened, etc. Consider an idle-timeout
   that shuts down an LSP process if no relevant file has been touched in
   N minutes.
4. **Bounded background work.** Any background thread (file watcher, search
   indexer, git-status poller) needs an explicit concurrency/resource
   budget — no unbounded thread-per-file-change spawning.
5. **Virtualized rendering.** File tree and any large list views must use
   virtual scrolling — never render thousands of DOM nodes for a large
   project tree.
6. **Memory budget as a first-class metric.** Track and periodically report
   (during development) RSS memory with a mid-size real project open
   (multiple PHP files across several ddev sites) plus ddev/OrbStack
   running alongside. Target: stay meaningfully lighter than PhpStorm's
   footprint in the same scenario — treat regressions against this budget
   as bugs.

## ddev / OrbStack integration — undecided, keep it pluggable

No decision has been made yet on how deep container integration should go.
Don't hard-code an approach. Design the file-access and "run a command"
layers so that:
- The simple case (edit files that ddev has mounted into the container,
  purely on the host filesystem) works with zero container-specific code
- A future container-aware mode (e.g. running `ddev exec` for linters,
  PHPStan, `composer`, etc. from inside the editor) can be added later
  without a rewrite — e.g. keep "run this shell command" behind a small
  internal abstraction rather than calling `Command::new()` ad hoc all over
  the codebase

Revisit this decision explicitly once the MVP is usable day-to-day and it's
clear which container-related pain points actually matter in practice.

## MVP feature set

- [ ] Open a folder as a project; lazy-loading file tree
- [ ] Tabbed editor using CodeMirror 6
- [ ] Syntax highlighting: PHP, JS/TS, HTML, Twig, SQL, JSON/YAML, Markdown
- [ ] LSP integration: PHP first (Intelephense or Phpactor), then JS/TS
  - Hover, go-to-definition, diagnostics
  - Class/method/property completion popup (see "Code completion" above) —
    the main day-to-day payoff of the LSP work
- [ ] Snippet expansion for common PHP blocks (switch/foreach/try-catch/
  etc.) plus a few real Drupal patterns, stored in a user-editable file
  (see "Snippets" above)
- [ ] Fuzzy file open (ripgrep/fd-backed)
- [ ] In-project text search (ripgrep-backed), respecting ignore rules
- [ ] Git status indicators in the file tree (modified/untracked/staged)
- [ ] Command palette for the above actions
- [ ] Persisted per-project settings (open tabs, ignore overrides, LSP
  choice)

Deferred until MVP is solid and actually in daily use:
- Xdebug/debugging support
- Multi-root workspaces
- Themes/extensions
- SQL LSP
- Twig LSP (if one doesn't already exist that's lightweight)

## Development conventions

- **Rust backend:** organize by capability (`fs/`, `lsp/`, `search/`,
  `watcher/`, `git/`), not by Tauri-command-per-file. Keep Tauri commands
  thin — they should call into these modules, not contain logic themselves.
- **Frontend:** keep state management minimal and explicit; avoid pulling in
  a heavy state library unless the app's complexity actually demands it.
- **Testing:** Rust unit tests for the fs/search/LSP-process-management
  logic; don't over-invest in frontend test infrastructure until the UI
  shape has stabilized.
- **Before adding a dependency (Rust or JS):** check its footprint. This
  project's entire premise is "lighter than the alternative" — a heavy
  dependency undermines the point. Prefer fewer, well-chosen crates/packages
  over convenience.

## Open questions to revisit as the project develops

- Intelephense vs Phpactor — pick based on measured memory use, not
  reputation
- Svelte vs SolidJS vs no framework — decide once the first screen is being
  built and the actual UI complexity is clearer
- How much ddev/OrbStack-specific integration is actually worth building
  (see above)
- Whether Twig needs a real LSP or highlighting is sufficient in practice
- Whether auto-import-on-completion is worth building, or is more friction
  than it saves in practice
