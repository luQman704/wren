import { ViewPlugin, gutter, GutterMarker, EditorView } from '@codemirror/view';
import type { ViewUpdate } from '@codemirror/view';
import { StateField, StateEffect, RangeSetBuilder } from '@codemirror/state';
import type { RangeSet } from '@codemirror/state';
import type { LspClient, LspCodeAction, LspDiagnostic, LspRange } from './client';
import { applyWorkspaceEdit } from './workspace';

// ── State ──────────────────────────────────────────────────────────

interface CodeActionState {
  line: number; // 1-based
  actions: LspCodeAction[];
}

const setActionsEffect = StateEffect.define<CodeActionState | null>();

const codeActionsField = StateField.define<CodeActionState | null>({
  create: () => null,
  update(value, tr) {
    for (const e of tr.effects) {
      if (e.is(setActionsEffect)) return e.value;
    }
    return value;
  },
});

// ── Lightbulb marker ───────────────────────────────────────────────

class LightbulbMarker extends GutterMarker {
  constructor(
    private actions: LspCodeAction[],
    private getView: (uri: string) => EditorView | null,
    private onUpdated?: (uri: string, content: string) => void
  ) {
    super();
  }

  override toDOM(): Node {
    const btn = document.createElement('button');
    btn.className = 'cm-lightbulb';
    btn.textContent = '💡';
    btn.title = 'Code actions available';
    btn.setAttribute('aria-label', 'Show code actions');

    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      this.showMenu(btn);
    });

    return btn;
  }

  private showMenu(anchor: HTMLElement) {
    // Remove any existing menu
    document.querySelectorAll('.cm-codeaction-menu').forEach((el) => el.remove());

    const menu = document.createElement('div');
    menu.className = 'cm-codeaction-menu';

    for (const action of this.actions) {
      const item = document.createElement('button');
      item.className = 'cm-codeaction-item';
      item.textContent = action.title;
      item.addEventListener('click', async () => {
        menu.remove();
        if (action.edit) {
          await applyWorkspaceEdit(action.edit, this.getView, this.onUpdated);
        }
      });
      menu.appendChild(item);
    }

    const rect = anchor.getBoundingClientRect();
    menu.style.position = 'fixed';
    menu.style.top = `${rect.bottom + 2}px`;
    menu.style.left = `${rect.left}px`;
    document.body.appendChild(menu);

    const dismiss = (ev: MouseEvent) => {
      if (!menu.contains(ev.target as Node)) {
        menu.remove();
        document.removeEventListener('click', dismiss);
      }
    };
    setTimeout(() => document.addEventListener('click', dismiss), 0);
  }
}

// ── Gutter ─────────────────────────────────────────────────────────

function buildLightbulbSet(
  view: EditorView,
  state: CodeActionState | null,
  getView: (uri: string) => EditorView | null,
  onUpdated?: (uri: string, content: string) => void
): RangeSet<GutterMarker> {
  const builder = new RangeSetBuilder<GutterMarker>();
  if (!state || !state.actions.length) return builder.finish();
  const lineN = state.line;
  if (lineN < 1 || lineN > view.state.doc.lines) return builder.finish();
  const pos = view.state.doc.line(lineN).from;
  builder.add(pos, pos, new LightbulbMarker(state.actions, getView, onUpdated));
  return builder.finish();
}

function codeActionGutter(
  getView: (uri: string) => EditorView | null,
  onUpdated?: (uri: string, content: string) => void
) {
  return gutter({
    class: 'cm-codeaction-gutter',
    markers(view) {
      const state = view.state.field(codeActionsField);
      return buildLightbulbSet(view, state, getView, onUpdated);
    },
  });
}

// ── Plugin ─────────────────────────────────────────────────────────

export function codeActionExtension(
  client: LspClient,
  uri: string,
  getDiagnosticsAtPos: (pos: number) => LspDiagnostic[],
  getView: (uri: string) => EditorView | null,
  onUpdated?: (uri: string, content: string) => void
) {
  return [
    codeActionsField,
    ViewPlugin.fromClass(
      class {
        private debounce: ReturnType<typeof setTimeout> | null = null;

        constructor(private view: EditorView) {}

        update(update: ViewUpdate) {
          if (!update.selectionSet && !update.docChanged) return;
          if (this.debounce) clearTimeout(this.debounce);
          this.debounce = setTimeout(() => this.fetchActions(), 600);
        }

        private async fetchActions() {
          const state = this.view.state;
          const pos = state.selection.main.head;
          const line = state.doc.lineAt(pos);

          const lspLine = line.number - 1;
          const lspChar = pos - line.from;

          const range: LspRange = {
            start: { line: lspLine, character: lspChar },
            end: { line: lspLine, character: lspChar },
          };

          const diags = getDiagnosticsAtPos(pos);

          try {
            const actions = await client.codeAction(uri, range, diags);
            if (!actions || actions.length === 0) {
              this.view.dispatch({ effects: setActionsEffect.of(null) });
              return;
            }
            this.view.dispatch({
              effects: setActionsEffect.of({ line: line.number, actions }),
            });
          } catch {
            this.view.dispatch({ effects: setActionsEffect.of(null) });
          }
        }

        destroy() {
          if (this.debounce) clearTimeout(this.debounce);
        }
      }
    ),
    codeActionGutter(getView, onUpdated),
    EditorView.theme({
      '.cm-codeaction-gutter': { width: '16px' },
      '.cm-lightbulb': {
        all: 'unset',
        cursor: 'pointer',
        fontSize: '11px',
        lineHeight: '1',
        display: 'block',
        width: '16px',
        textAlign: 'center',
      },
    }),
  ];
}

// ── Menu styles (global) ───────────────────────────────────────────

const styleId = 'wren-codeaction-styles';
if (!document.getElementById(styleId)) {
  const style = document.createElement('style');
  style.id = styleId;
  style.textContent = `
    .cm-codeaction-menu {
      background: #1e1e2e;
      border: 1px solid #333;
      border-radius: 4px;
      padding: 4px 0;
      z-index: 9999;
      min-width: 180px;
      box-shadow: 0 4px 12px rgba(0,0,0,0.4);
    }
    .cm-codeaction-item {
      all: unset;
      display: block;
      width: 100%;
      box-sizing: border-box;
      padding: 5px 12px;
      font-size: 12px;
      color: #cdd6f4;
      cursor: pointer;
      white-space: nowrap;
    }
    .cm-codeaction-item:hover {
      background: #313244;
    }
  `;
  document.head.appendChild(style);
}
