<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import type { SearchResult } from '$lib/types';

  interface Props {
    root: string | null;
    onopen: (path: string, filename: string, line: number) => void;
  }

  let { root, onopen }: Props = $props();

  let query = $state('');
  let caseSensitive = $state(false);
  let results = $state<SearchResult[]>([]);
  let searching = $state(false);
  let error = $state<string | null>(null);
  let searchTimer: ReturnType<typeof setTimeout> | null = null;

  // Group results by file path
  let grouped = $derived.by(() => {
    const map = new Map<string, SearchResult[]>();
    for (const r of results) {
      const list = map.get(r.path) ?? [];
      list.push(r);
      map.set(r.path, list);
    }
    return map;
  });

  $effect(() => {
    // Trigger search when query or caseSensitive changes
    query; caseSensitive;
    if (searchTimer) clearTimeout(searchTimer);
    if (!query.trim() || !root) {
      results = [];
      error = null;
      return;
    }
    searchTimer = setTimeout(runSearch, 300);
  });

  async function runSearch() {
    searching = true;
    error = null;
    try {
      results = await invoke<SearchResult[]>('search_project', {
        root,
        query: query.trim(),
        caseSensitive,
      });
    } catch (e) {
      error = String(e);
      results = [];
    } finally {
      searching = false;
    }
  }

  function handleOpen(r: SearchResult) {
    if (!root) return;
    const fullPath = `${root}/${r.path}`;
    const filename = r.path.split('/').pop() ?? r.path;
    onopen(fullPath, filename, r.line_number);
  }

  // Render a line with the match range highlighted
  function renderLine(line: string, start: number, end: number): string {
    const esc = (s: string) =>
      s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
    const trimmed = line.trimStart();
    const trimOffset = line.length - trimmed.length;
    const s = Math.max(0, start - trimOffset);
    const e = Math.max(s, end - trimOffset);
    const before = esc(trimmed.slice(0, s));
    const match = esc(trimmed.slice(s, e));
    const after = esc(trimmed.slice(e, 120));
    return `${before}<mark>${match}</mark>${after}`;
  }
</script>

<div class="search-panel">
  <div class="search-input-row">
    <input
      class="search-input"
      placeholder="Search in project…"
      bind:value={query}
      spellcheck={false}
    />
    <button
      class="case-btn"
      class:active={caseSensitive}
      onclick={() => (caseSensitive = !caseSensitive)}
      title="Match case"
    >Aa</button>
  </div>

  <div class="results">
    {#if !root}
      <p class="hint">Open a folder first</p>
    {:else if error}
      <p class="hint error">{error}</p>
    {:else if searching}
      <p class="hint">Searching…</p>
    {:else if query && results.length === 0}
      <p class="hint">No results</p>
    {:else if results.length >= 500}
      <p class="hint warn">500+ matches — refine your query</p>
    {/if}

    {#each [...grouped] as [path, matches]}
      <div class="file-group">
        <div class="file-header" title={path}>
          <span class="file-name">{path.split('/').pop()}</span>
          <span class="file-dir">{path.includes('/') ? path.slice(0, path.lastIndexOf('/')) : ''}</span>
          <span class="match-count">{matches.length}</span>
        </div>
        {#each matches as r (r.line_number)}
          <button class="match-row" onclick={() => handleOpen(r)}>
            <span class="line-num">{r.line_number}</span>
            <!-- eslint-disable-next-line svelte/no-at-html-tags -->
            <span class="line-text">{@html renderLine(r.line, r.match_start, r.match_end)}</span>
          </button>
        {/each}
      </div>
    {/each}
  </div>
</div>

<style>
  .search-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .search-input-row {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 8px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .search-input {
    all: unset;
    flex: 1;
    background: #1a1a1a;
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 4px 8px;
    font-size: 12px;
    color: var(--text);
    font-family: inherit;
  }

  .search-input:focus {
    border-color: var(--accent);
    outline: none;
  }

  .case-btn {
    all: unset;
    cursor: pointer;
    padding: 3px 6px;
    border-radius: 3px;
    font-size: 11px;
    color: var(--text-muted);
    border: 1px solid transparent;
    transition: all 0.1s;
  }

  .case-btn:hover { color: var(--text-dim); border-color: var(--border); }
  .case-btn.active { color: var(--accent); border-color: var(--accent); }

  .results {
    flex: 1;
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: #444 transparent;
  }

  .hint {
    padding: 16px 12px;
    font-size: 12px;
    color: var(--text-muted);
  }
  .hint.error { color: #f87171; }
  .hint.warn { color: #fbbf24; }

  .file-group {
    border-bottom: 1px solid #2a2a2a;
  }

  .file-header {
    display: flex;
    align-items: baseline;
    gap: 6px;
    padding: 5px 10px 3px;
    position: sticky;
    top: 0;
    background: #252525;
    border-bottom: 1px solid #2a2a2a;
  }

  .file-name {
    font-size: 12px;
    color: var(--text);
    font-weight: 500;
    flex-shrink: 0;
  }

  .file-dir {
    font-size: 10px;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  .match-count {
    margin-left: auto;
    font-size: 10px;
    color: var(--text-muted);
    background: #333;
    padding: 1px 5px;
    border-radius: 8px;
    flex-shrink: 0;
  }

  .match-row {
    all: unset;
    display: flex;
    align-items: baseline;
    gap: 8px;
    width: 100%;
    padding: 3px 10px;
    cursor: pointer;
    box-sizing: border-box;
    transition: background 0.08s;
  }

  .match-row:hover {
    background: #ffffff0a;
  }

  .line-num {
    font-size: 11px;
    color: var(--text-muted);
    min-width: 28px;
    text-align: right;
    flex-shrink: 0;
    font-variant-numeric: tabular-nums;
    font-family: 'JetBrains Mono', Menlo, monospace;
  }

  .line-text {
    font-size: 11px;
    color: var(--text-dim);
    font-family: 'JetBrains Mono', Menlo, monospace;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .line-text :global(mark) {
    background: none;
    color: #e2b86b;
    font-weight: 600;
  }
</style>
