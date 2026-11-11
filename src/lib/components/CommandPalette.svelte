<script lang="ts">
  import { tick } from 'svelte';

  interface Props {
    files: string[];           // relative paths from project root
    onopen: (rel: string) => void;
    onclose: () => void;
  }

  let { files, onopen, onclose }: Props = $props();

  let query = $state('');
  let selectedIdx = $state(0);
  let inputEl: HTMLInputElement;

  // Simple fuzzy filter: every char in query must appear in order in the path
  let filtered = $derived.by(() => {
    if (!query) return files.slice(0, 100);
    const q = query.toLowerCase();
    const results: { path: string; score: number }[] = [];
    for (const f of files) {
      const lower = f.toLowerCase();
      let qi = 0, score = 0, lastMatch = -1;
      for (let i = 0; i < lower.length && qi < q.length; i++) {
        if (lower[i] === q[qi]) {
          // Bonus for consecutive matches and filename matches
          score += (i === lastMatch + 1 ? 3 : 1);
          lastMatch = i;
          qi++;
        }
      }
      if (qi === q.length) {
        results.push({ path: f, score });
      }
    }
    return results
      .sort((a, b) => b.score - a.score)
      .slice(0, 50)
      .map((r) => r.path);
  });

  $effect(() => {
    // Reset selection when results change
    selectedIdx = 0;
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onclose();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIdx = Math.min(selectedIdx + 1, filtered.length - 1);
      scrollSelected();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIdx = Math.max(selectedIdx - 1, 0);
      scrollSelected();
    } else if (e.key === 'Enter') {
      if (filtered[selectedIdx]) {
        onopen(filtered[selectedIdx]);
      }
    }
  }

  async function scrollSelected() {
    await tick();
    document.querySelector('.palette-item.selected')?.scrollIntoView({ block: 'nearest' });
  }

  // Highlight matching chars in a path string
  function highlight(path: string, q: string): string {
    if (!q) return escHtml(path);
    const lower = path.toLowerCase();
    const ql = q.toLowerCase();
    let qi = 0;
    let out = '';
    for (let i = 0; i < path.length; i++) {
      if (qi < ql.length && lower[i] === ql[qi]) {
        out += `<mark>${escHtml(path[i])}</mark>`;
        qi++;
      } else {
        out += escHtml(path[i]);
      }
    }
    return out;
  }

  function escHtml(s: string) {
    return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="overlay" onclick={onclose} onkeydown={handleKeydown}>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="palette" onclick={(e) => e.stopPropagation()}>
    <!-- svelte-ignore a11y_autofocus -->
    <input
      bind:this={inputEl}
      bind:value={query}
      class="palette-input"
      placeholder="Go to file…"
      onkeydown={handleKeydown}
      autofocus
    />
    <div class="palette-results">
      {#if filtered.length === 0}
        <div class="palette-empty">No files match</div>
      {:else}
        {#each filtered as path, i (path)}
          <button
            class="palette-item"
            class:selected={i === selectedIdx}
            onclick={() => onopen(path)}
            onmouseenter={() => (selectedIdx = i)}
          >
            <span class="item-dir">{path.includes('/') ? path.slice(0, path.lastIndexOf('/') + 1) : ''}</span><!--
            --><span class="item-name" contenteditable="false">
              <!-- eslint-disable-next-line svelte/no-at-html-tags -->
              {@html highlight(path.includes('/') ? path.slice(path.lastIndexOf('/') + 1) : path, query)}
            </span>
          </button>
        {/each}
      {/if}
    </div>
    <div class="palette-hint">
      <span>↑↓ navigate</span>
      <span>↵ open</span>
      <span>esc close</span>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 80px;
    z-index: 1000;
  }

  .palette {
    width: 560px;
    max-width: calc(100vw - 40px);
    background: #252526;
    border: 1px solid #454545;
    border-radius: 6px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.6);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .palette-input {
    all: unset;
    width: 100%;
    padding: 12px 14px;
    font-size: 13px;
    color: var(--text);
    background: transparent;
    border-bottom: 1px solid var(--border);
    box-sizing: border-box;
    font-family: inherit;
  }

  .palette-input::placeholder {
    color: var(--text-muted);
  }

  .palette-results {
    max-height: 340px;
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: #444 transparent;
  }

  .palette-item {
    all: unset;
    display: flex;
    width: 100%;
    padding: 7px 14px;
    font-size: 12px;
    cursor: pointer;
    box-sizing: border-box;
    color: var(--text-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .palette-item:hover,
  .palette-item.selected {
    background: #04395e;
    color: var(--text);
  }

  .item-dir {
    color: var(--text-muted);
  }

  .item-name {
    color: var(--text);
  }

  .palette-item :global(mark) {
    background: none;
    color: #e2b86b;
    font-weight: 600;
  }

  .palette-empty {
    padding: 20px 14px;
    font-size: 12px;
    color: var(--text-muted);
  }

  .palette-hint {
    display: flex;
    gap: 14px;
    padding: 6px 14px;
    border-top: 1px solid var(--border);
    font-size: 11px;
    color: var(--text-muted);
  }
</style>
