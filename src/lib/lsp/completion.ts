import type { CompletionContext, CompletionResult } from '@codemirror/autocomplete';
import type { LspClient, LspCompletionItem, LspCompletionList } from './client';

// LSP completion item kinds → CM6 type labels
const KIND_LABELS: Record<number, string> = {
  1: 'text', 2: 'method', 3: 'function', 4: 'constructor',
  5: 'field', 6: 'variable', 7: 'class', 8: 'interface',
  9: 'module', 10: 'property', 14: 'keyword', 15: 'snippet',
  17: 'snippet', 21: 'constant',
};

// PHP/JS trigger sequences that warrant a TriggerCharacter completion request
const TRIGGER_SEQUENCES = ['->', '::', '\\', '.'];
const TRIGGER_CHARS = ['>', ':', '\\', '.'];  // last char of each sequence

function lspItemsToCm(items: LspCompletionItem[]) {
  return items.map((item) => ({
    label: item.label,
    type: KIND_LABELS[item.kind ?? 0] ?? 'text',
    detail: item.detail,
    info: typeof item.documentation === 'string'
      ? item.documentation
      : item.documentation?.value,
    // Strip LSP snippet placeholders for plain insertion
    apply: (item.insertText ?? item.label).replace(/\$\{[^}]+\}|\$\d+/g, ''),
    boost: item.kind === 2 || item.kind === 3 ? 1 : 0, // boost methods/functions
  }));
}

export function makeLspCompletionSource(client: LspClient, uri: string) {
  return async (ctx: CompletionContext): Promise<CompletionResult | null> => {
    const pos = ctx.pos;
    const line = ctx.state.doc.lineAt(pos);
    const lspLine = line.number - 1;
    const lspChar = pos - line.from;

    // Check for trigger characters (last 2 chars before cursor)
    const before2 = ctx.state.sliceDoc(Math.max(line.from, pos - 2), pos);
    const triggerSeq = TRIGGER_SEQUENCES.find(t => before2.endsWith(t));

    // Match word characters, including $ for PHP variables and \ for namespaces
    const word = ctx.matchBefore(/[$\\]?[\w][\w]*/);

    if (!triggerSeq && !word && !ctx.explicit) return null;

    // Determine LSP trigger kind:
    // 1 = Invoked (manual or word-typed), 2 = TriggerCharacter
    const triggerKind = triggerSeq ? 2 : 1;
    const triggerCharacter = triggerSeq ? triggerSeq[triggerSeq.length - 1] : undefined;

    try {
      const result = await client.completion(uri, lspLine, lspChar, triggerKind, triggerCharacter);
      if (!result) return null;

      const rawItems: LspCompletionItem[] = Array.isArray(result)
        ? result
        : (result as LspCompletionList).items;

      if (!rawItems.length) return null;

      // from = start of word, or current pos for trigger-only (no word yet)
      const from = word ? word.from : pos;

      return {
        from,
        options: lspItemsToCm(rawItems),
        validFor: /^[$\\]?[\w]*$/,
      };
    } catch {
      return null;
    }
  };
}
