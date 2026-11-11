<script lang="ts">
  import type { Tab } from '$lib/types';

  interface Props {
    tabs: Tab[];
    activeId: string;
    onactivate: (id: string) => void;
    onclose: (id: string) => void;
  }

  let { tabs, activeId, onactivate, onclose }: Props = $props();

  function handleClose(e: MouseEvent, id: string) {
    e.stopPropagation();
    onclose(id);
  }
</script>

<div class="tabbar">
  {#each tabs as tab (tab.id)}
    <div
      class="tab"
      class:active={tab.id === activeId}
      role="tab"
      tabindex="0"
      aria-selected={tab.id === activeId}
      title={tab.path}
      onclick={() => onactivate(tab.id)}
      onkeydown={(e) => e.key === 'Enter' && onactivate(tab.id)}
    >
      <span class="tab-name">{tab.filename}</span>
      {#if tab.modified}
        <span class="modified-dot" title="Unsaved changes"></span>
      {/if}
      <button
        class="close-btn"
        onclick={(e) => handleClose(e, tab.id)}
        title="Close"
        aria-label="Close {tab.filename}"
      >×</button>
    </div>
  {/each}
</div>

<style>
  .tabbar {
    display: flex;
    align-items: stretch;
    height: var(--tab-height);
    background: var(--bg-tabs);
    border-bottom: 1px solid var(--border);
    overflow-x: auto;
    scrollbar-width: none;
    flex-shrink: 0;
  }

  .tabbar::-webkit-scrollbar {
    display: none;
  }

  .tab {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 12px 0 14px;
    min-width: 0;
    max-width: 180px;
    border-right: 1px solid var(--border);
    color: var(--text-dim);
    font-size: 12px;
    white-space: nowrap;
    cursor: pointer;
    background: var(--bg-tabs);
    transition: background 0.1s, color 0.1s;
    flex-shrink: 0;
    user-select: none;
  }

  .tab:hover {
    background: var(--bg-tab-hover);
    color: var(--text);
  }

  .tab.active {
    background: var(--bg-tab-active);
    color: var(--text);
    border-bottom: 1px solid var(--bg-tab-active);
    margin-bottom: -1px;
  }

  .tab-name {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .modified-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--text-dim);
    flex-shrink: 0;
  }

  .tab.active .modified-dot {
    background: var(--accent);
  }

  .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    border-radius: 3px;
    font-size: 14px;
    line-height: 1;
    color: transparent;
    flex-shrink: 0;
    margin-left: 2px;
    transition: background 0.1s, color 0.1s;
  }

  .tab:hover .close-btn,
  .tab.active .close-btn {
    color: var(--text-dim);
  }

  .close-btn:hover {
    background: var(--hover-bg-btn);
    color: var(--text) !important;
  }
</style>
