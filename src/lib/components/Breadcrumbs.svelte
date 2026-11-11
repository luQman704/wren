<script lang="ts">
  import type { LspClient, LspDocumentSymbol } from '$lib/lsp/client';

  interface Props {
    lspClient: LspClient | null;
    fileUri: string;
    filename: string;
    cursorLine: number;
  }

  let { lspClient, fileUri, filename, cursorLine }: Props = $props();

  // Symbol cache: uri -> symbols
  const symbolCache = new Map<string, LspDocumentSymbol[]>();
  let symbols = $state<LspDocumentSymbol[]>([]);
  let debounce: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    const uri = fileUri;
    const client = lspClient;
    if (!client) {
      symbols = [];
      return;
    }

    if (symbolCache.has(uri)) {
      symbols = symbolCache.get(uri)!;
    } else {
      if (debounce) clearTimeout(debounce);
      debounce = setTimeout(async () => {
        try {
          const syms = await client.documentSymbol(uri);
          const result = syms ?? [];
          symbolCache.set(uri, result);
          symbols = result;
        } catch {
          symbols = [];
        }
      }, 300);
    }

    return () => {
      if (debounce) clearTimeout(debounce);
    };
  });

  // When cursorLine changes, just re-derive (symbols already loaded)
  // When fileUri changes, clear and reload
  $effect(() => {
    void fileUri; // reactive dep
    symbolCache.delete(fileUri);
    symbols = [];
  });

  function findBreadcrumbs(syms: LspDocumentSymbol[], line: number): string[] {
    for (const sym of syms) {
      const start = sym.range.start.line + 1;
      const end = sym.range.end.line + 1;
      if (line >= start && line <= end) {
        const childPath = sym.children ? findBreadcrumbs(sym.children, line) : [];
        return [sym.name, ...childPath];
      }
    }
    return [];
  }

  const breadcrumbs = $derived(findBreadcrumbs(symbols, cursorLine));
  const displayName = $derived(filename.split('/').pop() ?? filename);
</script>

<div class="breadcrumbs" role="navigation" aria-label="Symbol breadcrumbs">
  <span class="crumb filename">{displayName}</span>
  {#each breadcrumbs as crumb}
    <span class="separator">›</span>
    <span class="crumb">{crumb}</span>
  {/each}
</div>

<style>
  .breadcrumbs {
    display: flex;
    align-items: center;
    height: 20px;
    padding: 0 10px;
    background: #181825;
    border-bottom: 1px solid #2d2d3a;
    font-size: 11px;
    color: #888;
    flex-shrink: 0;
    overflow: hidden;
    gap: 4px;
    white-space: nowrap;
  }

  .crumb {
    color: #aaa;
  }

  .crumb.filename {
    color: #cdd6f4;
  }

  .separator {
    color: #555;
    flex-shrink: 0;
  }
</style>
