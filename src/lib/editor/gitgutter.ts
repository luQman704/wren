import { ViewPlugin, gutter, GutterMarker } from '@codemirror/view';
import type { EditorView, ViewUpdate } from '@codemirror/view';
import { StateField, StateEffect, RangeSetBuilder } from '@codemirror/state';
import type { RangeSet } from '@codemirror/state';
import { invoke } from '@tauri-apps/api/core';

// ── Types ──────────────────────────────────────────────────────────

interface DiffLine {
  line: number; // 1-based
  kind: 'add' | 'modify' | 'delete';
}

// ── State ──────────────────────────────────────────────────────────

const setDiffEffect = StateEffect.define<DiffLine[]>();

const diffState = StateField.define<DiffLine[]>({
  create: () => [],
  update(value, tr) {
    for (const e of tr.effects) {
      if (e.is(setDiffEffect)) return e.value;
    }
    return value;
  },
});

// ── Gutter markers ─────────────────────────────────────────────────

class DiffMarker extends GutterMarker {
  constructor(readonly kind: 'add' | 'modify' | 'delete') {
    super();
  }

  override toDOM(): Node {
    const el = document.createElement('div');
    el.className = `git-diff-marker git-diff-${this.kind}`;
    return el;
  }
}

const addMarker = new DiffMarker('add');
const modifyMarker = new DiffMarker('modify');
const deleteMarker = new DiffMarker('delete');

function markerFor(kind: 'add' | 'modify' | 'delete') {
  if (kind === 'add') return addMarker;
  if (kind === 'modify') return modifyMarker;
  return deleteMarker;
}

// ── Gutter definition ──────────────────────────────────────────────

function buildMarkerSet(view: EditorView, diffs: DiffLine[]): RangeSet<GutterMarker> {
  const builder = new RangeSetBuilder<GutterMarker>();
  // Sort by line number so we can build in order
  const sorted = [...diffs].sort((a, b) => a.line - b.line);
  for (const d of sorted) {
    if (d.line < 1 || d.line > view.state.doc.lines) continue;
    const pos = view.state.doc.line(d.line).from;
    builder.add(pos, pos, markerFor(d.kind));
  }
  return builder.finish();
}

function gitDiffGutterExt() {
  return gutter({
    class: 'cm-git-diff-gutter',
    markers(view) {
      const diffs = view.state.field(diffState);
      return buildMarkerSet(view, diffs);
    },
    initialSpacer: () => new DiffMarker('add'),
  });
}

// ── ViewPlugin: fetch diffs on change ──────────────────────────────

function gitDiffPlugin(root: string, filePath: string) {
  return ViewPlugin.fromClass(
    class {
      private debounce: ReturnType<typeof setTimeout> | null = null;

      constructor(private view: EditorView) {
        this.scheduleFetch();
      }

      update(update: ViewUpdate) {
        if (update.docChanged) {
          this.scheduleFetch();
        }
      }

      private scheduleFetch() {
        if (this.debounce) clearTimeout(this.debounce);
        this.debounce = setTimeout(() => this.fetchDiff(), 1000);
      }

      private async fetchDiff() {
        if (!root || !filePath) return;
        try {
          const diffs = await invoke<DiffLine[]>('git_diff', { root, path: filePath });
          this.view.dispatch({ effects: setDiffEffect.of(diffs) });
        } catch {
          // git not available or not a repo — silently ignore
        }
      }

      destroy() {
        if (this.debounce) clearTimeout(this.debounce);
      }
    }
  );
}

// ── Public API ─────────────────────────────────────────────────────

export function gitGutterExtension(root: string, filePath: string) {
  return [diffState, gitDiffPlugin(root, filePath), gitDiffGutterExt()];
}

// ── Styles (injected once) ─────────────────────────────────────────

const styleId = 'wren-git-gutter-styles';
if (!document.getElementById(styleId)) {
  const style = document.createElement('style');
  style.id = styleId;
  style.textContent = `
    .cm-git-diff-gutter { width: 4px; }
    .git-diff-marker { width: 4px; height: 100%; }
    .git-diff-add { background: #4ec994; }
    .git-diff-modify { background: #e5c07b; }
    .git-diff-delete { background: #e06c75; clip-path: polygon(0 50%, 100% 0, 100% 100%); }
  `;
  document.head.appendChild(style);
}
