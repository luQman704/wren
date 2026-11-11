import { hoverTooltip } from '@codemirror/view';
import type { LspClient } from './client';

export function makeLspHoverSource(client: LspClient, uri: string) {
  return hoverTooltip(async (view, pos) => {
    const line = view.state.doc.lineAt(pos);
    const lspLine = line.number - 1;
    const lspChar = pos - line.from;

    try {
      const result = await client.hover(uri, lspLine, lspChar);
      if (!result) return null;

      const content = typeof result.contents === 'string'
        ? result.contents
        : result.contents?.value ?? '';

      if (!content) return null;

      return {
        pos,
        end: pos,
        above: true,
        create() {
          const dom = document.createElement('div');
          dom.className = 'lsp-hover';
          dom.textContent = content;
          return { dom };
        },
      };
    } catch {
      return null;
    }
  });
}
