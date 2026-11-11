import { LspClient } from './client';

class LspManager {
  private clients = new Map<string, LspClient>();
  private errors = new Map<string, string>();
  private rootPath: string | null = null;

  setRoot(path: string) {
    this.rootPath = path;
  }

  languageForFile(filename: string): string | null {
    const ext = filename.split('.').pop()?.toLowerCase() ?? '';
    if (['php', 'theme', 'module', 'install', 'inc', 'engine'].includes(ext)) return 'php';
    if (['js', 'mjs', 'cjs', 'jsx'].includes(ext)) return 'javascript';
    if (['ts', 'tsx'].includes(ext)) return 'typescript';
    return null;
  }

  // lsp server key — typescript-language-server handles both js and ts
  serverKey(language: string): string {
    if (language === 'typescript') return 'javascript';
    return language;
  }

  async ensureStarted(filename: string): Promise<LspClient | null> {
    if (!this.rootPath) return null;
    const lang = this.languageForFile(filename);
    if (!lang) return null;

    const key = this.serverKey(lang);
    if (!this.clients.has(key)) {
      const client = new LspClient(key);
      // Insert a pending promise so concurrent callers wait for the same startup
      const starting = client.start(this.rootPath)
        .then(() => {
          this.clients.set(key, client);
          this.errors.delete(key);
        })
        .catch((e) => {
          console.error(`Failed to start LSP for ${key}:`, e);
          this.errors.set(key, `Failed to start ${key} LSP`);
        });
      // Store a sentinel so concurrent ensureStarted calls don't double-start
      this.clients.set(key, client);
      await starting;
      if (this.errors.has(key)) {
        this.clients.delete(key);
        return null;
      }
    }
    return this.clients.get(key) ?? null;
  }

  getClient(filename: string): LspClient | null {
    const lang = this.languageForFile(filename);
    if (!lang) return null;
    return this.clients.get(this.serverKey(lang)) ?? null;
  }

  getError(filename: string): string | null {
    const lang = this.languageForFile(filename);
    if (!lang) return null;
    return this.errors.get(this.serverKey(lang)) ?? null;
  }

  async stopAll() {
    for (const [, client] of this.clients) {
      await client.stop().catch(() => {});
    }
    this.clients.clear();
  }
}

export const lspManager = new LspManager();
