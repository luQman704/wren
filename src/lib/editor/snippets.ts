import { invoke } from '@tauri-apps/api/core';
import { snippet } from '@codemirror/autocomplete';
import type { CompletionContext, CompletionResult } from '@codemirror/autocomplete';

export interface SnippetDef {
  prefix: string;
  label: string;
  description: string;
  body: string;
}

// Language key used to load the right snippet file.
// Matches what Rust's load_snippets expects.
export function snippetLanguageKey(filename: string): string | null {
  const ext = filename.split('.').pop()?.toLowerCase() ?? '';
  if (ext === 'php') return 'php';
  if (['js', 'mjs', 'cjs', 'jsx', 'ts', 'tsx'].includes(ext)) return 'javascript';
  return null;
}

// Cache so we don't hit the Rust backend on every keystroke
const cache = new Map<string, SnippetDef[]>();

export async function loadSnippets(language: string): Promise<SnippetDef[]> {
  if (cache.has(language)) return cache.get(language)!;
  try {
    const raw = await invoke<string>('load_snippets', { language });
    const defs: SnippetDef[] = JSON.parse(raw);
    cache.set(language, defs);
    return defs;
  } catch (e) {
    console.error('load_snippets failed:', e);
    return [];
  }
}

export function makeSnippetCompletionSource(defs: SnippetDef[]) {
  return (ctx: CompletionContext): CompletionResult | null => {
    const word = ctx.matchBefore(/\w+/);
    if (!word) return null;
    if (word.text.length < 1) return null;

    const q = word.text.toLowerCase();
    const options = defs
      .filter((s) => s.prefix.toLowerCase().startsWith(q))
      .map((s) => ({
        label: s.prefix,
        displayLabel: s.prefix,
        detail: s.label,
        info: s.description,
        type: 'keyword',
        // snippet() handles ${1:placeholder} syntax and Tab-through navigation
        apply: snippet(s.body),
        // Boost slightly lower than LSP items so class names come first
        boost: -5,
      }));

    return options.length > 0 ? { from: word.from, options, validFor: /^\w*$/ } : null;
  };
}
