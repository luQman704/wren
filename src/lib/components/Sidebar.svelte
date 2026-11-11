<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { invoke } from '@tauri-apps/api/core';
  import { onMount, setContext } from 'svelte';
  import TreeNode from './TreeNode.svelte';
  import SearchPanel from './SearchPanel.svelte';
  import type { FsEntry, GitEntry } from '$lib/types';

  interface Props {
    onopen: (path: string, filename: string, line?: number) => void;
    onprojectopen?: (root: string) => void;
    ondelete?: (path: string) => void;
  }

  let { onopen, onprojectopen, ondelete }: Props = $props();

  type Mode = 'explorer' | 'search';
  let mode = $state<Mode>('explorer');

  let projectPath = $state<string | null>(null);
  let projectName = $derived(
    projectPath ? projectPath.split('/').filter(Boolean).pop() ?? projectPath : null
  );
  let rootEntries = $state<FsEntry[]>([]);

  // Git status: absolute path → status char
  let gitStatusMap = $state(new Map<string, string>());
  setContext('gitStatus', gitStatusMap);

  // Recent projects
  let recentProjects = $state<string[]>([]);

  // Resizable sidebar
  let sidebarWidth = $state(220);

  onMount(async () => {
    recentProjects = await invoke<string[]>('load_recent');
  });

  export async function openFolder() {
    const selected = await open({ directory: true, multiple: false });
    if (!selected || typeof selected !== 'string') return;
    projectPath = selected;
    onprojectopen?.(selected);
    const newRecent = [selected, ...recentProjects.filter((p) => p !== selected)];
    recentProjects = newRecent.slice(0, 10);
    invoke('save_recent', { paths: newRecent }).catch(() => {});
    await Promise.all([loadTree(selected), refreshGitStatus(selected)]);
  }

  async function loadTree(path: string) {
    try {
      rootEntries = await invoke<FsEntry[]>('read_dir', { path });
    } catch (e) {
      console.error('read_dir failed:', e);
    }
  }

  async function refreshGitStatus(root: string) {
    try {
      const entries = await invoke<GitEntry[]>('git_status', { root });
      const map = new Map<string, string>();
      for (const e of entries) {
        map.set(`${root}/${e.path}`, e.status);
      }
      gitStatusMap.clear();
      for (const [k, v] of map) gitStatusMap.set(k, v);
    } catch {
      // Not a git repo or git unavailable — silently ignore
    }
  }

  async function openRecent(path: string) {
    projectPath = path;
    onprojectopen?.(path);
    const newRecent = [path, ...recentProjects.filter((p) => p !== path)];
    recentProjects = newRecent;
    invoke('save_recent', { paths: newRecent }).catch(() => {});
    await Promise.all([loadTree(path), refreshGitStatus(path)]);
  }

  function startResize(e: MouseEvent) {
    const startX = e.clientX;
    const startW = sidebarWidth;
    const onMove = (ev: MouseEvent) => {
      sidebarWidth = Math.max(160, Math.min(500, startW + ev.clientX - startX));
    };
    const onUp = () => {
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
    };
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
  }

  // Expose a way for the page to switch to search mode (e.g. via Cmd+Shift+F)
  export function focusSearch() {
    mode = 'search';
  }


</script>

<aside class="sidebar" class:wide={mode === 'search'} style:width="{mode === 'search' ? 320 : sidebarWidth}px" style:min-width="{mode === 'search' ? 320 : sidebarWidth}px">
  <div class="sidebar-header">
    <div class="mode-tabs">
      <button
        class="mode-tab"
        class:active={mode === 'explorer'}
        onclick={() => (mode = 'explorer')}
        title="Explorer"
      >Explorer</button>
      <button
        class="mode-tab"
        class:active={mode === 'search'}
        onclick={() => (mode = 'search')}
        title="Search"
      >Search</button>
    </div>
    {#if mode === 'explorer' && projectPath}
      <button class="folder-btn" onclick={openFolder} title="Open folder">⊕</button>
    {/if}
  </div>

  <div class="sidebar-body">
    {#if mode === 'search'}
      <SearchPanel root={projectPath} onopen={(path, filename, line) => onopen(path, filename, line)} />
    {:else if rootEntries.length > 0}
      <div class="tree">
        {#each rootEntries as entry (entry.path)}
          <TreeNode
            {entry}
            depth={0}
            onopen={(path, filename) => onopen(path, filename)}
            {ondelete}
          />
        {/each}
      </div>
    {:else}
      <div class="empty-state">
        <p>No folder open</p>
        <button class="open-btn" onclick={openFolder}>Open Folder</button>
        {#if recentProjects.length > 0}
          <div class="recent-list">
            <p class="recent-label">Recent</p>
            {#each recentProjects as p}
              <button class="recent-item" onclick={() => openRecent(p)} title={p}>
                <span class="recent-name">{p.split('/').pop()}</span>
                <span class="recent-path">{p}</span>
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>

  {#if mode === 'explorer' && rootEntries.length > 0}
    <div class="sidebar-footer">
      <button class="open-btn-small" onclick={openFolder}>Open Folder…</button>
    </div>
  {/if}

  <!-- Resize handle (only visible in explorer mode) -->
  {#if mode === 'explorer'}
    <div class="resize-handle" onmousedown={startResize} role="separator" aria-label="Resize sidebar"></div>
  {/if}
</aside>

<style>
  .sidebar {
    background: var(--bg-sidebar);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    position: relative;
  }

  .sidebar.wide {
    width: 320px !important;
    min-width: 320px !important;
    transition: width 0.15s ease, min-width 0.15s ease;
  }

  .sidebar-header {
    padding: 0 8px 0 12px;
    height: var(--tab-height);
    display: flex;
    align-items: center;
    gap: 6px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .mode-tabs {
    display: flex;
    gap: 2px;
    flex: 1;
  }

  .mode-tab {
    all: unset;
    cursor: pointer;
    font-size: 11px;
    padding: 3px 8px;
    border-radius: 3px;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    transition: background 0.1s, color 0.1s;
  }

  .mode-tab:hover { color: var(--text-dim); }
  .mode-tab.active { color: var(--text); background: var(--hover-bg-btn); }

  .folder-btn {
    all: unset;
    cursor: pointer;
    font-size: 16px;
    line-height: 1;
    color: var(--text-muted);
    padding: 2px 4px;
    border-radius: 3px;
    transition: color 0.1s;
  }

  .folder-btn:hover { color: var(--text-dim); }

  .sidebar-body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: auto;
    padding: 4px 0;
    scrollbar-width: thin;
    scrollbar-color: #444 transparent;
  }

  /* Search panel takes full height, no extra padding */
  .sidebar.wide .sidebar-body {
    padding: 0;
    overflow: hidden;
  }

  .tree { min-width: 100%; width: max-content; }

  .empty-state {
    padding: 24px 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    align-items: flex-start;
  }

  .empty-state > p { color: var(--text-muted); font-size: 12px; }

  .open-btn {
    all: unset;
    cursor: pointer;
    padding: 5px 10px;
    background: var(--accent-dim);
    color: var(--text);
    border-radius: 4px;
    font-size: 12px;
    transition: background 0.1s;
  }

  .open-btn:hover { background: var(--accent); }

  .sidebar-footer {
    padding: 6px 10px;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
  }

  .open-btn-small {
    all: unset;
    cursor: pointer;
    font-size: 11px;
    color: var(--text-muted);
    transition: color 0.1s;
  }

  .open-btn-small:hover { color: var(--text-dim); }

  /* Recent projects */
  .recent-list {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .recent-label {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
    margin: 0 0 4px 0;
  }

  .recent-item {
    all: unset;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding: 5px 8px;
    border-radius: 4px;
    transition: background 0.1s;
    width: 100%;
    box-sizing: border-box;
    overflow: hidden;
  }

  .recent-item:hover { background: var(--hover-bg); }

  .recent-name {
    font-size: 12px;
    color: var(--text-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .recent-path {
    font-size: 10px;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* Resize handle */
  .resize-handle {
    position: absolute;
    right: 0;
    top: 0;
    bottom: 0;
    width: 4px;
    cursor: col-resize;
    z-index: 10;
  }

  .resize-handle:hover { background: var(--accent); opacity: 0.4; }
</style>
