import { setDiagnostics } from '@codemirror/lint';
import type { EditorView } from '@codemirror/view';
import type { LspDiagnostic } from './client';

const SEVERITY_MAP = {
  1: 'error',
  2: 'warning',
  3: 'info',
  4: 'hint',
} as const;

export function applyDiagnostics(view: EditorView, lspDiags: LspDiagnostic[]) {
  const doc = view.state.doc;
  const cmDiags = lspDiags.flatMap((d) => {
    try {
      const startLine = doc.line(d.range.start.line + 1);
      const endLine = doc.line(d.range.end.line + 1);
      const from = startLine.from + d.range.start.character;
      const to = endLine.from + d.range.end.character;

      if (from < 0 || to > doc.length || from > to) return [];

      return [{
        from,
        to: to === from ? from + 1 : to,
        severity: SEVERITY_MAP[d.severity ?? 1] ?? 'error',
        message: d.message,
        source: d.source,
      }];
    } catch {
      return [];
    }
  });

  view.dispatch(setDiagnostics(view.state, cmDiags));
}
