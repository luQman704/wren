import type { EditorView } from '@codemirror/view';
import type { LspWorkspaceEdit, LspTextEdit } from './client';
import { invoke } from '@tauri-apps/api/core';

/**
 * Apply a workspace edit (from rename / code action) to open editor views
 * and files on disk.
 *
 * @param edit        The LspWorkspaceEdit returned by the server
 * @param getView     Function returning the EditorView for a file URI, or null
 * @param onUpdated   Optional callback after each file is patched (uri, new content)
 */
export async function applyWorkspaceEdit(
  edit: LspWorkspaceEdit,
  getView: (uri: string) => EditorView | null,
  onUpdated?: (uri: string, newContent: string) => void
): Promise<void> {
  // Collect { uri -> edits[] } from both formats
  const fileEdits: Map<string, LspTextEdit[]> = new Map();

  if (edit.changes) {
    for (const [uri, edits] of Object.entries(edit.changes)) {
      fileEdits.set(uri, edits);
    }
  }

  if (edit.documentChanges) {
    for (const dc of edit.documentChanges) {
      fileEdits.set(dc.textDocument.uri, dc.edits);
    }
  }

  for (const [uri, edits] of fileEdits) {
    // Sort edits bottom-to-top so applying earlier edits doesn't shift offsets
    const sorted = [...edits].sort((a, b) => {
      const lineDiff = b.range.start.line - a.range.start.line;
      if (lineDiff !== 0) return lineDiff;
      return b.range.start.character - a.range.start.character;
    });

    const view = getView(uri);
    if (view) {
      // Apply to open editor
      const doc = view.state.doc;
      let changes: { from: number; to: number; insert: string }[] = [];

      for (const edit of sorted) {
        const startLine = doc.line(edit.range.start.line + 1);
        const endLine = doc.line(edit.range.end.line + 1);
        const from = startLine.from + edit.range.start.character;
        const to = endLine.from + edit.range.end.character;
        changes.push({ from, to, insert: edit.newText });
      }

      view.dispatch({ changes });

      const newContent = view.state.doc.toString();
      onUpdated?.(uri, newContent);
    } else {
      // File not open — read from disk, patch, write back
      const path = uri.replace(/^file:\/\//, '');
      try {
        let content = await invoke<string>('read_file', { path });
        // Apply edits from bottom to top (already sorted)
        const lines = content.split('\n');

        for (const e of sorted) {
          const startOff = lineCharToOffset(lines, e.range.start.line, e.range.start.character);
          const endOff = lineCharToOffset(lines, e.range.end.line, e.range.end.character);
          content = content.slice(0, startOff) + e.newText + content.slice(endOff);
          // Re-split for next edit (edits are bottom-to-top so this is safe)
        }

        await invoke('write_file', { path, content });
        onUpdated?.(uri, content);
      } catch (err) {
        console.error(`Failed to apply workspace edit to ${path}:`, err);
      }
    }
  }
}

function lineCharToOffset(lines: string[], line: number, character: number): number {
  let offset = 0;
  for (let i = 0; i < line && i < lines.length; i++) {
    offset += lines[i].length + 1; // +1 for \n
  }
  return offset + character;
}
