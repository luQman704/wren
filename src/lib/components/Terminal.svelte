<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';

  interface Props {
    cwd: string;
    onclose: () => void;
  }

  let { cwd, onclose }: Props = $props();

  let termContainer: HTMLDivElement;

  onMount(() => {
    let termId: number | null = null;
    let destroyed = false;
    const unlisteners: Array<() => void> = [];

    (async () => {
      // Dynamically import so Vite doesn't tree-shake these browser-only deps
      const { Terminal } = await import('xterm');
      const { FitAddon } = await import('xterm-addon-fit');

      if (destroyed) return;

      const term = new Terminal({
        theme: {
          background: '#11111b',
          foreground: '#cdd6f4',
          cursor: '#f5e0dc',
          cursorAccent: '#11111b',
          black: '#45475a',
          red: '#f38ba8',
          green: '#a6e3a1',
          yellow: '#f9e2af',
          blue: '#89b4fa',
          magenta: '#f5c2e7',
          cyan: '#94e2d5',
          white: '#bac2de',
          brightBlack: '#585b70',
          brightRed: '#f38ba8',
          brightGreen: '#a6e3a1',
          brightYellow: '#f9e2af',
          brightBlue: '#89b4fa',
          brightMagenta: '#f5c2e7',
          brightCyan: '#94e2d5',
          brightWhite: '#a6adc8',
        },
        fontFamily: "'JetBrains Mono', 'Fira Code', Menlo, monospace",
        fontSize: 12,
        lineHeight: 1.4,
        scrollback: 5000,
        cursorBlink: true,
      });

      const fitAddon = new FitAddon();
      term.loadAddon(fitAddon);
      term.open(termContainer);
      fitAddon.fit();

      // Create PTY
      const cols = term.cols;
      const rows = term.rows;
      try {
        termId = await invoke<number>('pty_create', { cols, rows, cwd });
      } catch (err) {
        term.write(`\x1b[31mFailed to start terminal: ${err}\x1b[0m\r\n`);
        return;
      }

      if (destroyed) {
        if (termId !== null) invoke('pty_kill', { id: termId }).catch(() => {});
        term.dispose();
        return;
      }

      // Listen for PTY output
      const unOutput = await listen<string>(`pty_output_${termId}`, (event) => {
        term.write(event.payload);
      });
      unlisteners.push(unOutput);

      // Listen for PTY exit
      const unExit = await listen(`pty_exit_${termId}`, () => {
        term.write('\r\n\x1b[90m[Process exited]\x1b[0m\r\n');
        onclose();
      });
      unlisteners.push(unExit);

      // Forward keyboard input
      term.onData((data) => {
        if (termId === null) return;
        invoke('pty_write', { id: termId, data }).catch(() => {});
      });

      // Resize observer
      const ro = new ResizeObserver(() => {
        if (destroyed) return;
        fitAddon.fit();
        if (termId !== null) {
          invoke('pty_resize', { id: termId, cols: term.cols, rows: term.rows }).catch(() => {});
        }
      });
      ro.observe(termContainer);

      return () => {
        ro.disconnect();
      };
    })();

    return () => {
      destroyed = true;
      for (const fn of unlisteners) fn();
      if (termId !== null) {
        invoke('pty_kill', { id: termId }).catch(() => {});
      }
    };
  });
</script>

<div class="terminal-wrap" bind:this={termContainer}></div>

<style>
  .terminal-wrap {
    width: 100%;
    height: 100%;
    background: #11111b;
    overflow: hidden;
  }

  .terminal-wrap :global(.xterm) {
    height: 100%;
    padding: 4px 8px;
  }

  .terminal-wrap :global(.xterm-viewport) {
    overflow-y: auto !important;
  }
</style>
