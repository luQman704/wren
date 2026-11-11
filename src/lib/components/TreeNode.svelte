<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { getContext } from 'svelte';
  import TreeNode from './TreeNode.svelte';
  import type { FsEntry } from '$lib/types';

  interface Props {
    entry: FsEntry;
    depth: number;
    onopen: (path: string, filename: string) => void;
    ondelete?: (path: string) => void;
  }

  let { entry, depth, onopen, ondelete }: Props = $props();

  // Git status map: absolute path → status char, provided by Sidebar via context
  const gitStatus = getContext<Map<string, string>>('gitStatus');

  let expanded = $state(false);
  let children = $state<FsEntry[] | null>(null);
  let loading = $state(false);
  let hovered = $state(false);

  // Local display state for rename (props are immutable in Svelte 5)
  let displayName = $state(entry.name);
  let displayPath = $state(entry.path);

  // Inline input state
  let creating = $state<'file' | 'dir' | null>(null);
  let newName = $state('');
  let renaming = $state(false);
  let renameName = $state('');

  let status = $derived(gitStatus?.get(displayPath) ?? null);

  async function toggle() {
    if (creating || renaming) return;
    if (!entry.is_dir) {
      onopen(displayPath, displayName);
      return;
    }
    expanded = !expanded;
    if (expanded && children === null) {
      await loadChildren();
    }
  }

  async function loadChildren() {
    loading = true;
    try {
      children = await invoke<FsEntry[]>('read_dir', { path: displayPath });
    } catch {
      children = [];
    } finally {
      loading = false;
    }
  }

  async function startCreate(type: 'file' | 'dir', e: MouseEvent) {
    e.stopPropagation();
    if (!expanded) {
      expanded = true;
      if (children === null) await loadChildren();
    }
    creating = type;
    newName = '';
  }

  async function commitCreate(e: KeyboardEvent) {
    if (e.key === 'Escape') { creating = null; return; }
    if (e.key !== 'Enter') return;
    if (!newName.trim()) { creating = null; return; }
    const path = `${displayPath}/${newName.trim()}`;
    try {
      if (creating === 'file') {
        await invoke('create_file', { path });
      } else {
        await invoke('create_dir', { path });
      }
      await loadChildren();
    } catch (err) {
      console.error(err);
    }
    creating = null;
  }

  function startRename(e: MouseEvent) {
    e.stopPropagation();
    renaming = true;
    renameName = displayName;
  }

  async function commitRename(e: KeyboardEvent) {
    if (e.key === 'Escape') { renaming = false; return; }
    if (e.key !== 'Enter') return;
    if (!renameName.trim() || renameName === displayName) { renaming = false; return; }
    const parent = displayPath.slice(0, displayPath.lastIndexOf('/'));
    const newPath = `${parent}/${renameName.trim()}`;
    try {
      await invoke('rename_path', { oldPath: displayPath, newPath });
      displayPath = newPath;
      displayName = renameName.trim();
    } catch (err) {
      console.error(err);
    }
    renaming = false;
  }

  async function handleDelete(e: MouseEvent) {
    e.stopPropagation();
    if (!confirm(`Delete "${displayName}"?`)) return;
    try {
      await invoke('delete_path', { path: displayPath });
      ondelete?.(displayPath);
    } catch (err) {
      console.error(err);
      alert(`Delete failed: ${err}`);
    }
  }
</script>

<div
  class="node"
  style:padding-left="{depth * 12 + 8}px"
  onmouseenter={() => (hovered = true)}
  onmouseleave={() => (hovered = false)}
>
  {#if renaming}
    <input
      class="inline-input"
      bind:value={renameName}
      onkeydown={commitRename}
      onblur={() => (renaming = false)}
      autofocus
    />
  {:else}
    <div
      class="node-row"
      class:is-dir={entry.is_dir}
      role="button"
      tabindex="0"
      onclick={toggle}
      onkeydown={(e) => e.key === 'Enter' && toggle()}
    >
      <span class="chevron">
        {#if entry.is_dir}{expanded ? '▾' : '▸'}{/if}
      </span>
      <span class="name">{displayName}</span>
      {#if loading}
        <span class="loading">…</span>
      {:else if status && !hovered}
        <span class="git-badge" data-status={status}>{status}</span>
      {/if}
      {#if hovered}
        <span class="actions" onclick={(e) => e.stopPropagation()} role="group">
          {#if entry.is_dir}
            <button class="action-btn" title="New file" onclick={(e) => startCreate('file', e)}>+f</button>
            <button class="action-btn" title="New folder" onclick={(e) => startCreate('dir', e)}>+d</button>
          {/if}
          <button class="action-btn" title="Rename" onclick={startRename}>✎</button>
          <button class="action-btn danger" title="Delete" onclick={handleDelete}>×</button>
        </span>
      {/if}
    </div>
  {/if}

  {#if expanded && children !== null}
    {#if creating}
      <div class="new-item-row" style:padding-left="{12}px">
        <input
          class="inline-input"
          bind:value={newName}
          placeholder={creating === 'file' ? 'filename.php' : 'folder-name'}
          onkeydown={commitCreate}
          onblur={() => (creating = null)}
          autofocus
        />
      </div>
    {/if}
    {#each children as child (child.path)}
      <TreeNode entry={child} depth={depth + 1} {onopen} {ondelete} />
    {/each}
    {#if children.length === 0 && !creating}
      <div class="empty" style:padding-left="{(depth + 1) * 12 + 8}px">empty</div>
    {/if}
  {/if}
</div>

<style>
  .node { width: 100%; }

  .node-row {
    display: flex;
    align-items: center;
    gap: 5px;
    height: 24px;
    cursor: pointer;
    border-radius: 3px;
    color: var(--text-dim);
    font-size: 12px;
    padding-right: 8px;
    white-space: nowrap;
    overflow: hidden;
  }

  .node-row:hover { background: var(--hover-bg); color: var(--text); }
  .node-row.is-dir { color: var(--text); }

  .chevron {
    font-size: 10px;
    width: 12px;
    flex-shrink: 0;
    color: var(--text-muted);
    text-align: center;
  }

  .name { overflow: hidden; text-overflow: ellipsis; flex: 1; min-width: 0; }
  .loading { color: var(--text-muted); font-size: 11px; }

  .git-badge {
    font-size: 10px;
    font-weight: 600;
    flex-shrink: 0;
    padding: 0 2px;
  }

  .git-badge[data-status="M"] { color: #e2c08d; }
  .git-badge[data-status="A"] { color: #73c991; }
  .git-badge[data-status="D"] { color: #f14c4c; }
  .git-badge[data-status="?"] { color: #4d9fec; }
  .git-badge[data-status="R"] { color: #e2c08d; }

  .empty {
    font-size: 11px;
    color: var(--text-muted);
    height: 22px;
    display: flex;
    align-items: center;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 1px;
    flex-shrink: 0;
    margin-left: auto;
  }

  .action-btn {
    all: unset;
    cursor: pointer;
    font-size: 11px;
    color: var(--text-muted);
    padding: 1px 4px;
    border-radius: 3px;
    line-height: 1;
    transition: background 0.1s, color 0.1s;
  }

  .action-btn:hover { background: var(--hover-bg-btn); color: var(--text); }
  .action-btn.danger:hover { background: #f14c4c33; color: #f14c4c; }

  .inline-input {
    all: unset;
    font-size: 12px;
    color: var(--text);
    background: var(--bg-input);
    border: 1px solid var(--accent-dim);
    border-radius: 3px;
    padding: 1px 6px;
    width: calc(100% - 16px);
    height: 20px;
    box-sizing: border-box;
  }

  .new-item-row {
    padding: 2px 8px 2px 0;
  }
</style>
