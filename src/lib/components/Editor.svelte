<script lang="ts">
  import { onMount } from 'svelte';
  import { EditorView, keymap, lineNumbers, highlightActiveLineGutter,
           highlightSpecialChars, drawSelection, dropCursor,
           rectangularSelection, crosshairCursor, highlightActiveLine } from '@codemirror/view';
  import type { EditorView as EditorViewType } from '@codemirror/view';
  import { EditorState } from '@codemirror/state';
  import { history, defaultKeymap, historyKeymap } from '@codemirror/commands';
  import { foldGutter, foldKeymap, indentOnInput, syntaxHighlighting,
           defaultHighlightStyle, bracketMatching } from '@codemirror/language';
  import { oneDark } from '@codemirror/theme-one-dark';
  import { githubLight } from '@uiw/codemirror-theme-github';
  import { tokyoNight } from '@uiw/codemirror-theme-tokyo-night';
  import { lintGutter } from '@codemirror/lint';
  import { autocompletion, closeBrackets, closeBracketsKeymap,
           completionKeymap } from '@codemirror/autocomplete';
  import { search, searchKeymap, gotoLine, highlightSelectionMatches } from '@codemirror/search';
  import { lintKeymap } from '@codemirror/lint';
  import { indentationMarkers } from '@replit/codemirror-indentation-markers';
  import { getLanguageExtension } from '$lib/editor/language';
  import { syntaxErrorLinter } from '$lib/editor/linter';
  import { gitGutterExtension } from '$lib/editor/gitgutter';
  import { applyDiagnostics } from '$lib/lsp/diagnostics';
  import { makeLspCompletionSource } from '$lib/lsp/completion';
  import { makeLspHoverSource } from '$lib/lsp/hover';
  import { signatureHelpExtension } from '$lib/lsp/signature';
  import { codeActionExtension } from '$lib/lsp/codeaction';
  import { applyWorkspaceEdit } from '$lib/lsp/workspace';
  import { loadSnippets, makeSnippetCompletionSource, snippetLanguageKey } from '$lib/editor/snippets';
  import type { LspClient, LspDiagnostic } from '$lib/lsp/client';

  // Custom base setup — identical to basicSetup but WITHOUT autocompletion(),
  // so we can add our own single autocompletion instance with LSP override.
  function makeBaseSetup() {
    return [
      lineNumbers(),
      highlightActiveLineGutter(),
      highlightSpecialChars(),
      history(),
      foldGutter(),
      drawSelection(),
      dropCursor(),
      EditorState.allowMultipleSelections.of(true),
      indentOnInput(),
      syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
      bracketMatching(),
      closeBrackets(),
      rectangularSelection(),
      crosshairCursor(),
      highlightActiveLine(),
      highlightSelectionMatches(),
      search({ top: false }),
      keymap.of([
        ...closeBracketsKeymap,
        ...defaultKeymap,
        ...searchKeymap,
        ...historyKeymap,
        ...foldKeymap,
        ...completionKeymap,
        ...lintKeymap,
      ]),
    ];
  }

  function getThemeExtension(theme: string) {
    switch (theme) {
      case 'github-light': return githubLight;
      case 'tokyo-night':  return tokyoNight;
      default:             return oneDark;
    }
  }

  interface Props {
    content: string;
    filename: string;
    fileUri: string;
    lspClient: LspClient | null;
    projectRoot?: string;
    startLine?: number;
    savedScrollTop?: number;
    theme?: string;
    fontSize?: number;
    onchange?: (content: string) => void;
    onsave?: (content: string) => void;
    ongotodefinition?: (uri: string, line: number, character: number) => void;
    oncursorchange?: (line: number) => void;
    onscrollsave?: (top: number) => void;
  }

  let {
    content, filename, fileUri, lspClient, projectRoot,
    startLine, savedScrollTop = 0, theme = 'one-dark', fontSize = 13,
    onchange, onsave, ongotodefinition, oncursorchange, onscrollsave,
  }: Props = $props();

  let container: HTMLDivElement;

  // ── Rename overlay state ──────────────────────────────────────────
  let renaming = $state(false);
  let renameValue = $state('');
  let renamePos = $state({ x: 0, y: 0 });
  let renameInput = $state<HTMLInputElement | undefined>(undefined);
  let editorViewRef: EditorView | null = null;

  function showRenameOverlay(view: EditorView) {
    const pos = view.state.selection.main.head;
    const coords = view.coordsAtPos(pos);
    if (!coords) return;
    const line = view.state.doc.lineAt(pos);
    const text = line.text;
    const col = pos - line.from;
    let start = col;
    let end = col;
    while (start > 0 && /\w/.test(text[start - 1])) start--;
    while (end < text.length && /\w/.test(text[end])) end++;
    renameValue = text.slice(start, end);
    renamePos = { x: coords.left, y: coords.bottom + 2 };
    renaming = true;
    setTimeout(() => { renameInput?.focus(); renameInput?.select(); }, 10);
  }

  async function commitRename(view: EditorView) {
    renaming = false;
    const newName = renameValue.trim();
    if (!newName || !lspClient) return;
    const pos = view.state.selection.main.head;
    const line = view.state.doc.lineAt(pos);
    try {
      const edit = await lspClient.rename(fileUri, line.number - 1, pos - line.from, newName);
      if (edit) await applyWorkspaceEdit(edit, (_uri) => editorViewRef);
    } catch (err) {
      console.error('Rename failed:', err);
    }
  }

  onMount(() => {
    let view: EditorView | null = null;
    let destroyed = false;
    // Capture prop values before any awaits so they're stable
    const initialScrollTop = savedScrollTop;

    (async () => {
      const snippetLang = snippetLanguageKey(filename);
      const snippetDefs = snippetLang ? await loadSnippets(snippetLang) : [];
      if (destroyed) return;

      const snippetSource = makeSnippetCompletionSource(snippetDefs);
      const completionSources = [
        ...(lspClient ? [makeLspCompletionSource(lspClient, fileUri)] : []),
        snippetSource,
      ];

      let currentLspDiags: LspDiagnostic[] = [];
      function getDiagsAtPos(pos: number): LspDiagnostic[] {
        return currentLspDiags.filter((d) => {
          try {
            const state = view?.state;
            if (!state) return false;
            const sl = state.doc.line(d.range.start.line + 1);
            const el = state.doc.line(d.range.end.line + 1);
            const from = sl.from + d.range.start.character;
            const to = el.from + d.range.end.character;
            return pos >= from && pos <= to;
          } catch { return false; }
        });
      }

      const lspExtensions = lspClient ? [
        makeLspHoverSource(lspClient, fileUri),
        signatureHelpExtension(lspClient, fileUri),
        codeActionExtension(lspClient, fileUri, getDiagsAtPos, (_uri) => view),
      ] : [];

      const filePath = fileUri.replace('file://', '');

      view = new EditorView({
        state: EditorState.create({
          doc: content,
          extensions: [
            // Base setup (no autocompletion — we add one below)
            makeBaseSetup(),
            // Theme
            getThemeExtension(theme),
            // Language
            getLanguageExtension(filename),
            // Indentation guides
            indentationMarkers(),
            // Git diff gutter
            ...(projectRoot ? gitGutterExtension(projectRoot, filePath) : []),
            // Lint
            lintGutter(),
            syntaxErrorLinter(),
            // Single autocompletion with LSP + snippet sources
            autocompletion({
              override: completionSources,
              activateOnTyping: true,
              maxRenderedOptions: 30,
              defaultKeymap: false, // already in base keymap above
            }),
            // LSP extensions
            ...lspExtensions,
            // Custom keymaps
            keymap.of([
              { key: 'Mod-g', run: gotoLine },
              {
                key: 'Mod-s',
                run: (v: EditorViewType) => {
                  onsave?.(v.state.doc.toString());
                  return true;
                },
              },
              {
                key: 'F12',
                run: (v: EditorViewType) => {
                  if (!lspClient || !ongotodefinition) return false;
                  const pos = v.state.selection.main.head;
                  const line = v.state.doc.lineAt(pos);
                  lspClient.definition(fileUri, line.number - 1, pos - line.from)
                    .then((result) => {
                      if (!result) return;
                      const loc = Array.isArray(result) ? result[0] : result;
                      if (!loc) return;
                      ongotodefinition(loc.uri, loc.range.start.line + 1, loc.range.start.character);
                    }).catch(() => {});
                  return true;
                },
              },
              {
                key: 'F2',
                run: (v: EditorViewType) => {
                  if (!lspClient) return false;
                  showRenameOverlay(v as unknown as EditorView);
                  return true;
                },
              },
            ]),
            // Update listener
            EditorView.updateListener.of((update) => {
              if (update.docChanged) onchange?.(update.state.doc.toString());
              if (update.selectionSet || update.docChanged) {
                const pos = update.state.selection.main.head;
                oncursorchange?.(update.state.doc.lineAt(pos).number);
              }
            }),
            // Layout + typography overrides (no colors — let the theme handle those)
            EditorView.theme({
              '&': { height: '100%', fontSize: `${fontSize}px` },
              '.cm-scroller': {
                overflow: 'auto',
                fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', Menlo, monospace",
                lineHeight: '1.6',
              },
              '.cm-focused': { outline: 'none' },
              '.cm-tooltip.lsp-hover': {
                padding: '6px 10px',
                fontSize: '12px',
                maxWidth: '500px',
                whiteSpace: 'pre-wrap',
              },
            }),
          ],
        }),
        parent: container,
      });

      editorViewRef = view;

      if (startLine && startLine > 1) {
        const targetLine = Math.min(startLine, view.state.doc.lines);
        const pos = view.state.doc.line(targetLine).from;
        view.dispatch({
          selection: { anchor: pos },
          scrollIntoView: true,
          effects: EditorView.scrollIntoView(pos, { y: 'center' }),
        });
      } else if (initialScrollTop > 0) {
        // Restore previous scroll position for this tab
        requestAnimationFrame(() => {
          if (!destroyed && view) view.scrollDOM.scrollTop = initialScrollTop;
        });
      }

      if (lspClient) {
        lspClient.onNotification('textDocument/publishDiagnostics', (params: unknown) => {
          if (!view) return;
          const p = params as { uri: string; diagnostics: LspDiagnostic[] };
          if (p.uri === fileUri) {
            currentLspDiags = p.diagnostics ?? [];
            applyDiagnostics(view, p.diagnostics as Parameters<typeof applyDiagnostics>[1]);
          }
        });
      }
    })();

    return () => {
      // Save scroll position before the editor is destroyed (tab switch)
      if (view?.scrollDOM) onscrollsave?.(view.scrollDOM.scrollTop);
      destroyed = true;
      editorViewRef = null;
      view?.destroy();
    };
  });
</script>

<div class="editor-wrap" bind:this={container}></div>

{#if renaming}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="rename-overlay"
    style="left: {renamePos.x}px; top: {renamePos.y}px;"
    role="dialog"
    tabindex="-1"
    aria-label="Rename symbol"
    onkeydown={(e) => { if (e.key === 'Escape') { renaming = false; e.preventDefault(); } }}
  >
    <input
      bind:this={renameInput}
      bind:value={renameValue}
      class="rename-input"
      placeholder="New name"
      onkeydown={async (e) => {
        if (e.key === 'Enter') { e.preventDefault(); if (editorViewRef) await commitRename(editorViewRef); }
        else if (e.key === 'Escape') { e.preventDefault(); renaming = false; }
      }}
    />
  </div>
{/if}

<style>
  .editor-wrap {
    width: 100%;
    height: 100%;
    position: relative;
  }

  .editor-wrap :global(.cm-editor) { height: 100%; }

  .rename-overlay {
    position: fixed;
    z-index: 8000;
    background: #1e1e2e;
    border: 1px solid #7f849c;
    border-radius: 4px;
    padding: 4px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.5);
  }

  .rename-input {
    background: transparent;
    border: none;
    outline: none;
    color: #cdd6f4;
    font-family: 'JetBrains Mono', Menlo, monospace;
    font-size: 13px;
    padding: 2px 6px;
    min-width: 180px;
  }
</style>
