import { ViewPlugin, showTooltip, EditorView } from '@codemirror/view';
import type { ViewUpdate, Tooltip } from '@codemirror/view';
import { StateField, StateEffect } from '@codemirror/state';
import type { LspClient, LspSignatureHelp } from './client';

// ── State ──────────────────────────────────────────────────────────

const setSigEffect = StateEffect.define<Tooltip | null>();

const sigTooltipField = StateField.define<readonly Tooltip[]>({
  create: () => [],
  update(tooltips, tr) {
    for (const e of tr.effects) {
      if (e.is(setSigEffect)) {
        return e.value ? [e.value] : [];
      }
    }
    return tooltips;
  },
  provide: (f) => showTooltip.computeN([f], (state) => state.field(f)),
});

// ── Build tooltip DOM ──────────────────────────────────────────────

function buildSigDom(help: LspSignatureHelp): HTMLElement {
  const dom = document.createElement('div');
  dom.className = 'lsp-sig-help';

  const activeIdx = help.activeSignature ?? 0;
  const sig = help.signatures[activeIdx];
  if (!sig) return dom;

  const activeParam = help.activeParameter ?? sig.parameters?.length ? (help.activeParameter ?? 0) : -1;

  const label = document.createElement('div');
  label.className = 'lsp-sig-label';

  if (sig.parameters && sig.parameters.length > 0) {
    // Render label with the active parameter bolded
    const sigLabel = sig.label;
    let consumed = 0;

    sig.parameters.forEach((param, idx) => {
      let start: number;
      let end: number;

      if (Array.isArray(param.label)) {
        [start, end] = param.label as [number, number];
      } else {
        const offset = sigLabel.indexOf(param.label as string, consumed);
        start = offset >= 0 ? offset : consumed;
        end = start + (param.label as string).length;
      }

      // Text before this parameter
      if (start > consumed) {
        label.appendChild(document.createTextNode(sigLabel.slice(consumed, start)));
      }

      const span = document.createElement('span');
      span.textContent = sigLabel.slice(start, end);
      if (idx === activeParam) {
        span.style.fontWeight = 'bold';
        span.style.color = '#e5c07b';
      }
      label.appendChild(span);
      consumed = end;
    });

    // Remaining text after last param
    if (consumed < sigLabel.length) {
      label.appendChild(document.createTextNode(sigLabel.slice(consumed)));
    }
  } else {
    label.textContent = sig.label;
  }

  dom.appendChild(label);

  // Documentation
  if (sig.documentation) {
    const doc = document.createElement('div');
    doc.className = 'lsp-sig-doc';
    doc.textContent =
      typeof sig.documentation === 'string' ? sig.documentation : sig.documentation.value;
    dom.appendChild(doc);
  }

  return dom;
}

// ── Plugin ─────────────────────────────────────────────────────────

export function signatureHelpExtension(client: LspClient, uri: string) {
  return [
    sigTooltipField,
    ViewPlugin.fromClass(
      class {
        private pending = false;

        constructor(private view: EditorView) {}

        update(update: ViewUpdate) {
          if (!update.docChanged && !update.selectionSet) return;

          const state = update.state;
          const pos = state.selection.main.head;
          const line = state.doc.lineAt(pos);
          const charBefore = pos > line.from ? state.doc.sliceString(pos - 1, pos) : '';

          if (charBefore === '(' || charBefore === ',') {
            this.requestSignature(pos, line.number - 1, pos - line.from);
          } else if (charBefore === ')') {
            this.dismiss();
          }
        }

        private async requestSignature(pos: number, lspLine: number, lspChar: number) {
          if (this.pending) return;
          this.pending = true;
          try {
            const help = await client.signatureHelp(uri, lspLine, lspChar);
            if (!help || !help.signatures.length) {
              this.view.dispatch({ effects: setSigEffect.of(null) });
              return;
            }
            const tooltip: Tooltip = {
              pos,
              above: true,
              strictSide: false,
              arrow: false,
              create() {
                const dom = buildSigDom(help);
                return { dom };
              },
            };
            this.view.dispatch({ effects: setSigEffect.of(tooltip) });
          } catch {
            // ignore
          } finally {
            this.pending = false;
          }
        }

        private dismiss() {
          this.view.dispatch({ effects: setSigEffect.of(null) });
        }
      }
    ),
    EditorView.domEventHandlers({
      keydown(e, view) {
        if (e.key === 'Escape') {
          view.dispatch({ effects: setSigEffect.of(null) });
        }
      },
    }),
    EditorView.theme({
      '.lsp-sig-help': {
        padding: '6px 10px',
        fontSize: '12px',
        fontFamily: "'JetBrains Mono', Menlo, monospace",
        maxWidth: '600px',
        background: '#1e1e2e',
        border: '1px solid #333',
        borderRadius: '4px',
        color: '#cdd6f4',
      },
      '.lsp-sig-label': {
        whiteSpace: 'pre-wrap',
      },
      '.lsp-sig-doc': {
        marginTop: '4px',
        color: '#888',
        fontSize: '11px',
        whiteSpace: 'pre-wrap',
      },
    }),
  ];
}
