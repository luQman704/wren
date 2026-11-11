import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

type MessageHandler = (msg: unknown) => void;

export class LspClient {
  private nextId = 2; // 1 is used by initialize
  private pending = new Map<number, { resolve: (v: unknown) => void; reject: (e: unknown) => void }>();
  private notificationHandlers = new Map<string, MessageHandler[]>();
  private unlisten: UnlistenFn | null = null;
  readonly language: string;

  // Resolves once we've received the initialize response and sent initialized
  private _initializedResolve!: () => void;
  private _initialized = new Promise<void>(r => { this._initializedResolve = r; });

  constructor(language: string) {
    this.language = language;
  }

  async start(rootPath: string): Promise<void> {
    // Subscribe to incoming messages BEFORE starting, so we don't miss the initialize response
    this.unlisten = await listen<string>(`lsp_message_${this.language}`, (event) => {
      this.handleMessage(event.payload);
    });
    await invoke('lsp_start', { language: this.language, rootPath });
    // Wait until initialize handshake is complete before returning.
    // This ensures didOpen / completion requests are never sent to an un-initialized server.
    const timeout = new Promise<never>((_, reject) =>
      setTimeout(() => reject(new Error(`LSP ${this.language} init timeout`)), 15000)
    );
    await Promise.race([this._initialized, timeout]);
  }

  async stop(): Promise<void> {
    try {
      await invoke('lsp_stop', { language: this.language });
    } finally {
      this.unlisten?.();
      this.unlisten = null;
    }
  }

  private handleMessage(raw: string): void {
    let msg: Record<string, unknown>;
    try {
      msg = JSON.parse(raw);
    } catch {
      return;
    }

    if ('id' in msg && msg.id !== null) {
      const id = msg.id as number;
      const pending = this.pending.get(id);
      if (pending) {
        this.pending.delete(id);
        if ('error' in msg) {
          pending.reject(msg.error);
        } else {
          pending.resolve(msg.result);
        }
      }
      // Initialize response: complete the handshake then signal ready
      if (id === 1) {
        this.notify('initialized', {});
        this._initializedResolve();
      }
    } else if ('method' in msg) {
      // Notification from server
      const method = msg.method as string;
      const handlers = this.notificationHandlers.get(method) ?? [];
      for (const h of handlers) {
        h(msg.params);
      }
    }
  }

  onNotification(method: string, handler: MessageHandler): void {
    const existing = this.notificationHandlers.get(method) ?? [];
    this.notificationHandlers.set(method, [...existing, handler]);
  }

  async request(method: string, params: unknown): Promise<unknown> {
    const id = this.nextId++;
    const message = JSON.stringify({ jsonrpc: '2.0', id, method, params });
    const result = new Promise<unknown>((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
    });
    await invoke('lsp_send', { language: this.language, message });
    return result;
  }

  async notify(method: string, params: unknown): Promise<void> {
    const message = JSON.stringify({ jsonrpc: '2.0', method, params });
    await invoke('lsp_send', { language: this.language, message });
  }

  // ── Convenience wrappers ──────────────────────────────────────────

  didOpen(uri: string, languageId: string, version: number, text: string) {
    return this.notify('textDocument/didOpen', {
      textDocument: { uri, languageId, version, text },
    });
  }

  didChange(uri: string, version: number, text: string) {
    return this.notify('textDocument/didChange', {
      textDocument: { uri, version },
      contentChanges: [{ text }],
    });
  }

  didClose(uri: string) {
    return this.notify('textDocument/didClose', {
      textDocument: { uri },
    });
  }

  hover(uri: string, line: number, character: number) {
    return this.request('textDocument/hover', {
      textDocument: { uri },
      position: { line, character },
    }) as Promise<LspHover | null>;
  }

  completion(uri: string, line: number, character: number, triggerKind = 1, triggerCharacter?: string) {
    return this.request('textDocument/completion', {
      textDocument: { uri },
      position: { line, character },
      context: { triggerKind, ...(triggerCharacter ? { triggerCharacter } : {}) },
    }) as Promise<LspCompletionList | LspCompletionItem[] | null>;
  }

  definition(uri: string, line: number, character: number) {
    return this.request('textDocument/definition', {
      textDocument: { uri },
      position: { line, character },
    }) as Promise<LspLocation | LspLocation[] | null>;
  }

  signatureHelp(uri: string, line: number, character: number) {
    return this.request('textDocument/signatureHelp', {
      textDocument: { uri },
      position: { line, character },
    }) as Promise<LspSignatureHelp | null>;
  }

  rename(uri: string, line: number, character: number, newName: string) {
    return this.request('textDocument/rename', {
      textDocument: { uri },
      position: { line, character },
      newName,
    }) as Promise<LspWorkspaceEdit | null>;
  }

  codeAction(uri: string, range: LspRange, diagnostics: LspDiagnostic[]) {
    return this.request('textDocument/codeAction', {
      textDocument: { uri },
      range,
      context: { diagnostics },
    }) as Promise<LspCodeAction[] | null>;
  }

  documentSymbol(uri: string) {
    return this.request('textDocument/documentSymbol', {
      textDocument: { uri },
    }) as Promise<LspDocumentSymbol[] | null>;
  }
}

// ── Minimal LSP types ──────────────────────────────────────────────

export interface LspPosition { line: number; character: number }
export interface LspRange { start: LspPosition; end: LspPosition }
export interface LspLocation { uri: string; range: LspRange }

export interface LspDiagnostic {
  range: LspRange;
  severity?: 1 | 2 | 3 | 4; // error, warning, info, hint
  message: string;
  source?: string;
}

export interface LspHover {
  contents: { kind: string; value: string } | string;
  range?: LspRange;
}

export interface LspCompletionItem {
  label: string;
  kind?: number;
  detail?: string;
  documentation?: string | { kind: string; value: string };
  insertText?: string;
  insertTextFormat?: 1 | 2; // 1=plaintext, 2=snippet
}

export interface LspCompletionList {
  isIncomplete: boolean;
  items: LspCompletionItem[];
}

// ── Signature help ─────────────────────────────────────────────────

export interface LspSignatureHelp {
  signatures: LspSignatureInformation[];
  activeSignature?: number;
  activeParameter?: number;
}

export interface LspSignatureInformation {
  label: string;
  documentation?: string | { kind: string; value: string };
  parameters?: { label: string | [number, number]; documentation?: string }[];
}

// ── Workspace edits / rename ───────────────────────────────────────

export interface LspTextEdit {
  range: LspRange;
  newText: string;
}

export interface LspWorkspaceEdit {
  changes?: Record<string, LspTextEdit[]>;
  documentChanges?: Array<{ textDocument: { uri: string }; edits: LspTextEdit[] }>;
}

// ── Code actions ───────────────────────────────────────────────────

export interface LspCodeAction {
  title: string;
  kind?: string;
  edit?: LspWorkspaceEdit;
  command?: { title: string; command: string; arguments?: unknown[] };
}

// ── Document symbols ───────────────────────────────────────────────

export interface LspDocumentSymbol {
  name: string;
  kind: number;
  range: LspRange;
  selectionRange: LspRange;
  children?: LspDocumentSymbol[];
}
