<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import '../app.css';
  import Editor from '$lib/components/Editor.svelte';
  import TabBar from '$lib/components/TabBar.svelte';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import CommandPalette from '$lib/components/CommandPalette.svelte';
  import Breadcrumbs from '$lib/components/Breadcrumbs.svelte';
  import FileChangedToast from '$lib/components/FileChangedToast.svelte';
  import Terminal from '$lib/components/Terminal.svelte';
  import Preferences from '$lib/components/Preferences.svelte';
  import { getLanguageLabel } from '$lib/editor/language';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { lspManager } from '$lib/lsp/manager';
  import type { Tab, Session } from '$lib/types';

  let tabs = $state<Tab[]>([]);
  let activeId = $state<string | null>(null);
  let sidebarOpen = $state(true);
  let nextId = 1;
  let sessionLoaded = false;

  // Transient per-tab scroll target (cleared after first render)
  let startLines = new Map<string, number>();
  // Persisted scroll position per tab — saved on tab switch, restored on re-activate
  const scrollPositions = new Map<string, number>();

  // Command palette
  let paletteOpen = $state(false);
  let projectRoot = $state<string | null>(null);
  let fileList = $state<string[]>([]);

  // Sidebar component ref — used to programmatically switch to search mode
  let sidebarRef = $state<ReturnType<typeof Sidebar> | undefined>(undefined);

  // Debounce timers for LSP didChange
  let didChangeTimers = new Map<string, ReturnType<typeof setTimeout>>();

  // Format on save
  let formatOnSave = $state(false);

  // Auto-save
  let autoSave = $state(true);
  let autoSaveDelay = $state(1500);
  let autoSaveTimer: ReturnType<typeof setTimeout> | null = null;

  // Font size (display-only for now, emitted via prefs)
  let fontSize = $state(13);

  // Sidebar width (synced from prefs)
  let sidebarWidth = $state(220);

  // Intelephense license key
  let intelephenseLicenseKey = $state('');

  // Theme
  let currentTheme = $state('one-dark');

  // Apply theme to document root so CSS variables work app-wide
  $effect(() => {
    document.documentElement.setAttribute('data-theme', currentTheme);
  });

  // Preferences modal
  let prefsOpen = $state(false);

  // Cursor line (for Breadcrumbs)
  let cursorLine = $state(1);

  // Terminal panel
  let terminalOpen = $state(false);
  let terminalHeight = $state(200);
  let terminalKey = $state(0); // remount on re-open

  // File changed toast
  interface FileChangedNotice {
    tabId: string;
    path: string;
    filename: string;
  }
  let fileChangedNotice = $state<FileChangedNotice | null>(null);
  let watcherUnlisten: (() => void) | null = null;
  // Paths we wrote ourselves — suppress watcher toasts for 3s after our own saves
  const recentlySavedByUs = new Map<string, number>();

  // Menu event unlisteners
  let menuUnlisteners: Array<() => void> = [];

  let activeTab = $derived(tabs.find((t) => t.id === activeId) ?? null);
  let terminalCwd = $derived(projectRoot ?? '~');

  function fileUri(path: string): string {
    return `file://${path}`;
  }

  // ── Prefs persistence ────────────────────────────────────────────

  let prefsSaveTimer: ReturnType<typeof setTimeout> | null = null;

  function schedulePrefsSave() {
    if (prefsSaveTimer) clearTimeout(prefsSaveTimer);
    prefsSaveTimer = setTimeout(async () => {
      try {
        await invoke('save_prefs', {
          prefs: {
            auto_save: autoSave,
            auto_save_delay_ms: autoSaveDelay,
            format_on_save: formatOnSave,
            sidebar_width: sidebarWidth,
            intelephense_license_key: intelephenseLicenseKey || null,
            font_size: fontSize,
            theme: currentTheme,
          }
        });
      } catch (e) {
        console.warn('save_prefs failed:', e);
      }
    }, 500);
  }

  function applyPrefsChange(patch: Partial<{
    autoSave: boolean;
    autoSaveDelay: number;
    formatOnSave: boolean;
    fontSize: number;
    sidebarWidth: number;
    intelephenseLicenseKey: string;
    theme: string;
  }>) {
    if (patch.autoSave !== undefined) autoSave = patch.autoSave;
    if (patch.autoSaveDelay !== undefined) autoSaveDelay = patch.autoSaveDelay;
    if (patch.formatOnSave !== undefined) formatOnSave = patch.formatOnSave;
    if (patch.fontSize !== undefined) fontSize = patch.fontSize;
    if (patch.sidebarWidth !== undefined) sidebarWidth = patch.sidebarWidth;
    if (patch.intelephenseLicenseKey !== undefined) intelephenseLicenseKey = patch.intelephenseLicenseKey;
    if (patch.theme !== undefined) currentTheme = patch.theme;
    schedulePrefsSave();
  }

  // ── Keyboard shortcuts ───────────────────────────────────────────

  function handleGlobalKeydown(e: KeyboardEvent) {
    const mod = e.metaKey || e.ctrlKey;
    if (mod && e.key === 'p') {
      e.preventDefault();
      paletteOpen = true;
    }
    if (mod && e.shiftKey && e.key === 'F') {
      e.preventDefault();
      sidebarOpen = true;
      sidebarRef?.focusSearch();
    }
    // Ctrl+` toggles terminal
    if (e.ctrlKey && e.key === '`') {
      e.preventDefault();
      terminalOpen = !terminalOpen;
    }
    // Cmd+, opens preferences
    if (mod && e.key === ',') {
      e.preventDefault();
      prefsOpen = true;
    }
  }

  // ── Project open ─────────────────────────────────────────────────

  async function handleProjectOpen(root: string) {
    projectRoot = root;
    lspManager.setRoot(root);
    try {
      fileList = await invoke<string[]>('list_files', { root });
    } catch (err) {
      console.error('list_files failed:', err);
    }
    // Load existing session for this root
    await tryLoadSession(root);

    // Start file watcher
    startWatcher(root);
  }

  async function startWatcher(root: string) {
    // Clean up old watcher
    watcherUnlisten?.();
    watcherUnlisten = null;

    try {
      await invoke('watch_project', { root });

      watcherUnlisten = await listen<{ path: string }>('file_changed', (event) => {
        const changedPath = event.payload.path;
        // Ignore changes caused by our own saves (within the last 3 seconds)
        const savedAt = recentlySavedByUs.get(changedPath);
        if (savedAt) {
          if (Date.now() - savedAt < 3000) return; // still within suppress window
          recentlySavedByUs.delete(changedPath);   // expired — clean up
        }
        const tab = tabs.find((t) => t.path === changedPath);
        if (tab) {
          fileChangedNotice = { tabId: tab.id, path: tab.path, filename: tab.filename };
        }
      });
    } catch (err) {
      console.warn('File watcher unavailable:', err);
    }
  }

  async function handleReloadChangedFile() {
    if (!fileChangedNotice) return;
    const notice = fileChangedNotice;
    fileChangedNotice = null;

    try {
      const newContent = await invoke<string>('read_file', { path: notice.path });
      tabs = tabs.map((t) =>
        t.id === notice.tabId ? { ...t, content: newContent, modified: false } : t
      );
    } catch (err) {
      console.error('reload failed:', err);
    }
  }

  // ── Session persistence ──────────────────────────────────────────

  async function tryLoadSession(root: string) {
    try {
      const raw = await invoke<string | null>('load_session', { root });
      if (!raw) return;
      const session: Session = JSON.parse(raw);
      for (const { path, filename } of session.openTabs) {
        await openFile(path, filename);
      }
      if (session.activeTabPath) {
        const t = tabs.find((t) => t.path === session.activeTabPath);
        if (t) activeId = t.id;
      }
    } catch {
      // Corrupt session — ignore
    } finally {
      sessionLoaded = true;
    }
  }

  // Save session whenever open tabs change (debounced)
  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  $effect(() => {
    // Track tabs and projectRoot reactively
    const root = projectRoot;
    const snapshot = tabs.map((t) => ({ path: t.path, filename: t.filename }));
    const activePath = activeTab?.path ?? null;

    if (!root || !sessionLoaded) return;

    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(async () => {
      const session: Session = { root, openTabs: snapshot, activeTabPath: activePath };
      try {
        await invoke('save_session', { root, data: JSON.stringify(session) });
      } catch {
        // Non-critical
      }
    }, 1000);
  });

  // ── File open ────────────────────────────────────────────────────

  async function openFile(path: string, filename: string, line?: number) {
    const existing = tabs.find((t) => t.path === path);
    if (existing) {
      activeId = existing.id;
      if (line) startLines.set(existing.id, line);
      return;
    }
    try {
      const content = await invoke<string>('read_file', { path });
      const id = String(nextId++);
      tabs = [...tabs, { id, path, filename, content, modified: false }];
      activeId = id;
      if (line) startLines.set(id, line);

      const client = await lspManager.ensureStarted(filename);
      if (client) {
        const lang = lspManager.languageForFile(filename) ?? filename.split('.').pop() ?? 'text';
        await client.didOpen(fileUri(path), lang, 1, content);
      }
    } catch (e) {
      console.error('openFile failed:', e);
    }
  }

  // ── Tab management ───────────────────────────────────────────────

  function activateTab(id: string) {
    activeId = id;
  }

  function closeTab(id: string) {
    const idx = tabs.findIndex((t) => t.id === id);
    if (idx === -1) return;
    const tab = tabs[idx];
    lspManager.getClient(tab.filename)?.didClose(fileUri(tab.path)).catch(() => {});
    startLines.delete(id);
    tabs = tabs.filter((t) => t.id !== id);
    if (activeId === id) {
      activeId = tabs.length > 0 ? tabs[Math.max(0, idx - 1)].id : null;
    }
  }

  // Close any tab whose path was deleted in the file tree
  function handleDeletedPath(path: string) {
    const tab = tabs.find((t) => t.path === path || t.path.startsWith(path + '/'));
    if (tab) closeTab(tab.id);
    // Remove all tabs under deleted directory
    const toClose = tabs.filter((t) => t.path === path || t.path.startsWith(path + '/'));
    for (const t of toClose) closeTab(t.id);
  }

  // ── Editor callbacks ─────────────────────────────────────────────

  function handleContentChange(newContent: string) {
    if (!activeId || !activeTab) return;
    tabs = tabs.map((t) =>
      t.id === activeId ? { ...t, content: newContent, modified: true } : t
    );
    const tabId = activeId;
    const existing = didChangeTimers.get(tabId);
    if (existing) clearTimeout(existing);
    didChangeTimers.set(tabId, setTimeout(() => {
      didChangeTimers.delete(tabId);
      const tab = tabs.find((t) => t.id === tabId);
      if (!tab) return;
      lspManager.getClient(tab.filename)
        ?.didChange(fileUri(tab.path), Date.now(), newContent)
        .catch(() => {});
    }, 300));

    // Auto-save
    if (autoSave) {
      if (autoSaveTimer) clearTimeout(autoSaveTimer);
      autoSaveTimer = setTimeout(() => {
        const tab = tabs.find((t) => t.id === tabId);
        if (tab?.modified) handleSave(tab.content, tab);
      }, autoSaveDelay);
    }
  }

  async function handleSave(content: string, tab?: typeof activeTab) {
    const t = tab ?? activeTab;
    if (!t) return;
    let toWrite = content;
    if (formatOnSave) {
      const lang = lspManager.languageForFile(t.filename);
      if (lang) {
        try {
          toWrite = await invoke<string>('format_file', {
            path: t.path,
            content,
            language: lang,
          });
          if (toWrite !== content) {
            tabs = tabs.map((tab) =>
              tab.id === t.id ? { ...tab, content: toWrite } : tab
            );
          }
        } catch (e) {
          console.warn('Format failed:', e);
        }
      }
    }
    try {
      recentlySavedByUs.set(t.path, Date.now());
      await invoke('write_file', { path: t.path, content: toWrite });
      tabs = tabs.map((tab) => (tab.id === t.id ? { ...tab, modified: false } : tab));
    } catch (e) {
      console.error('write_file failed:', e);
    }
  }

  // ── Go to definition ─────────────────────────────────────────────

  function handleGoToDefinition(uri: string, line: number, _character: number) {
    // uri is file:///absolute/path/to/file.php
    const path = uri.replace(/^file:\/\//, '');
    const filename = path.split('/').pop() ?? path;
    openFile(path, filename, line);
  }

  // ── Command palette ──────────────────────────────────────────────

  async function openFromPalette(rel: string) {
    paletteOpen = false;
    if (!projectRoot) return;
    const path = `${projectRoot}/${rel}`;
    const filename = rel.split('/').pop() ?? rel;
    await openFile(path, filename);
  }

  // ── Mount / destroy ─────────────────────────────────────────────

  onMount(async () => {
    // Load prefs
    try {
      const p = await invoke<{
        auto_save: boolean;
        auto_save_delay_ms: number;
        format_on_save: boolean;
        sidebar_width: number;
        intelephense_license_key: string | null;
        font_size: number;
        theme: string;
      }>('load_prefs');
      autoSave = p.auto_save;
      autoSaveDelay = p.auto_save_delay_ms;
      formatOnSave = p.format_on_save;
      sidebarWidth = p.sidebar_width;
      intelephenseLicenseKey = p.intelephense_license_key ?? '';
      fontSize = p.font_size;
      currentTheme = p.theme ?? 'one-dark';
    } catch (e) {
      console.warn('load_prefs failed:', e);
    }

    // Listen for menu events from Rust
    const unlisteners = await Promise.all([
      listen('menu_preferences', () => { prefsOpen = true; }),
      listen('menu_open_folder', () => { sidebarRef?.openFolder(); }),
      listen('menu_close_tab', () => { if (activeId) closeTab(activeId); }),
      listen('menu_save', () => { if (activeTab) handleSave(activeTab.content); }),
      listen('menu_toggle_sidebar', () => { sidebarOpen = !sidebarOpen; }),
      listen('menu_toggle_terminal', () => { terminalOpen = !terminalOpen; }),
      listen('menu_command_palette', () => { paletteOpen = true; }),
      listen('menu_search', () => { sidebarOpen = true; sidebarRef?.focusSearch(); }),
      listen('menu_find', () => { /* CodeMirror search panel — handled by Editor keybinding */ }),
    ]);
    menuUnlisteners = unlisteners.map((u) => u);
  });

  onDestroy(() => {
    for (const unlisten of menuUnlisteners) unlisten();
  });

  // ── Terminal resizing ─────────────────────────────────────────────

  let isResizingTerminal = false;
  let resizeStartY = 0;
  let resizeStartHeight = 0;

  function startTerminalResize(e: MouseEvent) {
    isResizingTerminal = true;
    resizeStartY = e.clientY;
    resizeStartHeight = terminalHeight;
    e.preventDefault();
  }

  function onMouseMove(e: MouseEvent) {
    if (!isResizingTerminal) return;
    const delta = resizeStartY - e.clientY;
    terminalHeight = Math.max(80, Math.min(600, resizeStartHeight + delta));
  }

  function onMouseUp() {
    isResizingTerminal = false;
  }
</script>

<svelte:window
  onkeydown={handleGlobalKeydown}
  onmousemove={onMouseMove}
  onmouseup={onMouseUp}
/>

<div class="app">
  <div class="titlebar">
    <button
      class="sidebar-toggle"
      onclick={() => (sidebarOpen = !sidebarOpen)}
      title={sidebarOpen ? 'Hide sidebar' : 'Show sidebar'}
      aria-label="Toggle sidebar"
    >
      <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
        <rect x="1" y="2" width="12" height="1.5" rx="0.75" fill="currentColor"/>
        <rect x="1" y="6.25" width="12" height="1.5" rx="0.75" fill="currentColor"/>
        <rect x="1" y="10.5" width="12" height="1.5" rx="0.75" fill="currentColor"/>
      </svg>
    </button>
    <span class="app-name">Wren</span>
  </div>

  <div class="workspace">
    {#if sidebarOpen}
      <Sidebar
        bind:this={sidebarRef}
        onopen={(path, filename, line) => openFile(path, filename, line)}
        onprojectopen={handleProjectOpen}
        ondelete={handleDeletedPath}
      />
    {/if}

    <div class="editor-area">
      {#if tabs.length > 0 && activeTab}
        <TabBar
          {tabs}
          activeId={activeId ?? ''}
          onactivate={activateTab}
          onclose={closeTab}
        />

        <Breadcrumbs
          lspClient={lspManager.getClient(activeTab.filename)}
          fileUri={`file://${activeTab.path}`}
          filename={activeTab.filename}
          {cursorLine}
        />

        <div class="editor-pane">
          {#key `${activeId}|${currentTheme}|${fontSize}`}
            <Editor
              content={activeTab.content}
              filename={activeTab.filename}
              fileUri={`file://${activeTab.path}`}
              lspClient={lspManager.getClient(activeTab.filename)}
              projectRoot={projectRoot ?? undefined}
              startLine={startLines.get(activeId ?? '')}
              savedScrollTop={scrollPositions.get(activeId ?? '') ?? 0}
              theme={currentTheme}
              {fontSize}
              onchange={handleContentChange}
              onsave={handleSave}
              ongotodefinition={handleGoToDefinition}
              oncursorchange={(line) => { cursorLine = line; }}
              onscrollsave={(top) => { if (activeId) scrollPositions.set(activeId, top); }}
            />
          {/key}
        </div>

        {#if terminalOpen}
          <div class="terminal-panel" style="height: {terminalHeight}px;">
            <div
              class="terminal-resizer"
              role="separator"
              aria-label="Drag to resize terminal"
              onmousedown={startTerminalResize}
            ></div>
            <div class="terminal-header">
              <span class="terminal-title">Terminal</span>
              <button
                class="terminal-close"
                onclick={() => (terminalOpen = false)}
                title="Close terminal"
                aria-label="Close terminal"
              >✕</button>
            </div>
            <div class="terminal-body">
              {#key terminalKey}
                <Terminal
                  cwd={terminalCwd}
                  onclose={() => (terminalOpen = false)}
                />
              {/key}
            </div>
          </div>
        {/if}

        <div class="statusbar">
          <span class="statusbar-path">{activeTab.path}</span>
          <span class="statusbar-right">
            <button
              class="statusbar-btn"
              onclick={() => { terminalOpen = !terminalOpen; if (terminalOpen) terminalKey++; }}
              title="Toggle terminal (Ctrl+`)"
            >
              term
            </button>
            <button
              class="statusbar-btn"
              class:active={autoSave}
              onclick={() => applyPrefsChange({ autoSave: !autoSave })}
              title={autoSave ? 'Auto-save on (click to disable)' : 'Auto-save off (click to enable)'}
            >
              {autoSave ? 'auto-save' : 'manual'}
            </button>
            <button
              class="statusbar-btn"
              class:active={formatOnSave}
              onclick={() => applyPrefsChange({ formatOnSave: !formatOnSave })}
              title={formatOnSave ? 'Format on save on (click to disable)' : 'Format on save off (click to enable)'}
            >
              {formatOnSave ? 'fmt:on' : 'fmt:off'}
            </button>
            {getLanguageLabel(activeTab.filename)}
            {#if lspManager.getClient(activeTab.filename)}
              <span class="lsp-ok" title="LSP active">●</span>
            {:else if lspManager.getError(activeTab.filename)}
              <span class="lsp-err" title={lspManager.getError(activeTab.filename) ?? undefined}>⚠ LSP</span>
            {/if}
            {#if activeTab.modified}
              · <span class="unsaved">unsaved</span>
            {/if}
            · Ln {cursorLine}
          </span>
        </div>
      {:else}
        <div class="welcome">
          <p>Open a folder to get started · <kbd>⌘P</kbd> file · <kbd>⌘⇧F</kbd> search</p>
        </div>
      {/if}
    </div>
  </div>
</div>

{#if paletteOpen}
  <CommandPalette
    files={fileList}
    onopen={openFromPalette}
    onclose={() => (paletteOpen = false)}
  />
{/if}

{#if fileChangedNotice}
  <FileChangedToast
    filename={fileChangedNotice.filename}
    onreload={handleReloadChangedFile}
    ondismiss={() => { fileChangedNotice = null; }}
  />
{/if}

{#if prefsOpen}
  <Preferences
    {autoSave}
    {autoSaveDelay}
    {formatOnSave}
    {fontSize}
    {sidebarWidth}
    {intelephenseLicenseKey}
    theme={currentTheme}
    onclose={() => (prefsOpen = false)}
    onprefschange={applyPrefsChange}
  />
{/if}

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg-app);
    overflow: hidden;
  }

  .titlebar {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 38px;
    padding: 0 14px;
    background: var(--bg-sidebar);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    -webkit-app-region: drag;
    user-select: none;
  }

  .titlebar button, .sidebar-toggle { -webkit-app-region: no-drag; }

  .sidebar-toggle {
    color: var(--text-dim);
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border-radius: 4px;
    transition: background 0.1s, color 0.1s;
    cursor: pointer;
  }

  .sidebar-toggle:hover { background: #ffffff14; color: var(--text); }

  .app-name {
    font-size: 12px;
    color: var(--text-muted);
    letter-spacing: 0.04em;
  }

  .workspace { display: flex; flex: 1; overflow: hidden; }

  .editor-area {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow: hidden;
    min-width: 0;
  }

  .editor-pane { flex: 1; overflow: hidden; min-height: 0; }

  /* ── Terminal panel ── */
  .terminal-panel {
    display: flex;
    flex-direction: column;
    background: var(--bg-terminal);
    border-top: 1px solid var(--border-light);
    flex-shrink: 0;
    min-height: 80px;
  }

  .terminal-resizer {
    height: 4px;
    cursor: ns-resize;
    background: transparent;
    flex-shrink: 0;
  }
  .terminal-resizer:hover { background: #7f849c44; }

  .terminal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 26px;
    padding: 0 10px;
    background: var(--bg-term-header);
    border-bottom: 1px solid var(--border-light);
    flex-shrink: 0;
  }

  .terminal-title {
    font-size: 11px;
    color: #888;
    user-select: none;
  }

  .terminal-close {
    all: unset;
    cursor: pointer;
    font-size: 11px;
    color: #888;
    padding: 2px 6px;
    border-radius: 3px;
  }
  .terminal-close:hover { background: #ffffff14; color: #cdd6f4; }

  .terminal-body {
    flex: 1;
    overflow: hidden;
    min-height: 0;
  }

  /* ── Status bar ── */
  .statusbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 22px;
    padding: 0 10px;
    background: var(--statusbar-bg);
    font-size: 11px;
    color: var(--statusbar-text);
    flex-shrink: 0;
    gap: 8px;
  }

  .statusbar-path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
    color: var(--statusbar-text-dim);
  }

  .statusbar-right {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .unsaved { color: #ffcc66; }

  :global(.lsp-ok) { color: #a6e3a1; font-size: 10px; }
  :global(.lsp-err) { color: #f38ba8; font-size: 11px; cursor: help; }

  .statusbar-btn {
    all: unset;
    cursor: pointer;
    opacity: 0.7;
    font-size: 11px;
    padding: 0 4px;
    border-radius: 2px;
  }

  .statusbar-btn:hover { opacity: 1; }
  .statusbar-btn.active { color: #73c991; opacity: 1; }

  .welcome {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
    font-size: 13px;
  }

  .welcome kbd {
    font-family: inherit;
    background: #333;
    border: 1px solid #555;
    border-radius: 3px;
    padding: 1px 5px;
    font-size: 11px;
    color: var(--text-dim);
  }
</style>
