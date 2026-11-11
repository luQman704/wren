<script lang="ts">
  interface Props {
    autoSave: boolean;
    autoSaveDelay: number;
    formatOnSave: boolean;
    fontSize: number;
    sidebarWidth: number;
    intelephenseLicenseKey: string;
    theme: string;
    onclose: () => void;
    onprefschange: (patch: Partial<{
      autoSave: boolean;
      autoSaveDelay: number;
      formatOnSave: boolean;
      fontSize: number;
      sidebarWidth: number;
      intelephenseLicenseKey: string;
      theme: string;
    }>) => void;
  }

  const themes = [
    { id: 'one-dark',     label: 'One Dark' },
    { id: 'github-light', label: 'GitHub Light' },
    { id: 'tokyo-night',  label: 'Tokyo Night' },
  ];

  let {
    autoSave,
    autoSaveDelay,
    formatOnSave,
    fontSize,
    sidebarWidth,
    intelephenseLicenseKey,
    theme,
    onclose,
    onprefschange,
  }: Props = $props();

  type Section = 'editor' | 'lsp' | 'about';
  let activeSection = $state<Section>('editor');
  let showLicenseKey = $state(false);

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onclose();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="overlay" onclick={(e) => { if (e.target === e.currentTarget) onclose(); }}>
  <div class="modal" role="dialog" aria-modal="true" aria-label="Preferences">
    <!-- Left sidebar -->
    <nav class="prefs-nav">
      <div class="prefs-nav-title">Preferences</div>
      <button
        class="nav-item"
        class:active={activeSection === 'editor'}
        onclick={() => (activeSection = 'editor')}
      >Editor</button>
      <button
        class="nav-item"
        class:active={activeSection === 'lsp'}
        onclick={() => (activeSection = 'lsp')}
      >LSP</button>
      <button
        class="nav-item"
        class:active={activeSection === 'about'}
        onclick={() => (activeSection = 'about')}
      >About</button>
    </nav>

    <!-- Content area -->
    <div class="prefs-content">
      <button class="close-btn" onclick={onclose} aria-label="Close preferences">✕</button>

      {#if activeSection === 'editor'}
        <h2 class="section-title">Editor</h2>

        <div class="pref-group">
          <div class="pref-row">
            <div class="pref-label">
              <span class="pref-name">Auto-save</span>
              <span class="pref-desc">Automatically save files after a delay</span>
            </div>
            <button
              class="toggle"
              class:on={autoSave}
              onclick={() => onprefschange({ autoSave: !autoSave })}
              role="switch"
              aria-checked={autoSave}
              aria-label="Auto-save"
            >
              <span class="toggle-thumb"></span>
            </button>
          </div>

          {#if autoSave}
            <div class="pref-row pref-row-indent">
              <div class="pref-label">
                <span class="pref-name">Auto-save delay</span>
                <span class="pref-desc">{autoSaveDelay}ms</span>
              </div>
              <input
                type="range"
                min="500"
                max="5000"
                step="500"
                value={autoSaveDelay}
                oninput={(e) => onprefschange({ autoSaveDelay: Number((e.target as HTMLInputElement).value) })}
                class="slider"
              />
            </div>
          {/if}
        </div>

        <div class="pref-group">
          <div class="pref-row">
            <div class="pref-label">
              <span class="pref-name">Format on save</span>
              <span class="pref-desc">Run formatter (php-cs-fixer / prettier) when saving</span>
            </div>
            <button
              class="toggle"
              class:on={formatOnSave}
              onclick={() => onprefschange({ formatOnSave: !formatOnSave })}
              role="switch"
              aria-checked={formatOnSave}
              aria-label="Format on save"
            >
              <span class="toggle-thumb"></span>
            </button>
          </div>
        </div>

        <div class="pref-group">
          <div class="pref-row">
            <div class="pref-label">
              <span class="pref-name">Font size</span>
              <span class="pref-desc">Editor font size in pixels</span>
            </div>
            <div class="font-size-options">
              {#each [11, 12, 13, 14, 16] as size}
                <button
                  class="size-btn"
                  class:active={fontSize === size}
                  onclick={() => onprefschange({ fontSize: size })}
                >{size}</button>
              {/each}
            </div>
          </div>
        </div>

        <div class="pref-group">
          <div class="pref-row">
            <div class="pref-label">
              <span class="pref-name">Theme</span>
              <span class="pref-desc">Editor color theme</span>
            </div>
            <div class="theme-options">
              {#each themes as t}
                <button
                  class="theme-btn"
                  class:active={theme === t.id}
                  onclick={() => onprefschange({ theme: t.id })}
                >{t.label}</button>
              {/each}
            </div>
          </div>
        </div>

        <div class="pref-group">
          <div class="pref-row">
            <div class="pref-label">
              <span class="pref-name">Sidebar width</span>
              <span class="pref-desc">Currently {sidebarWidth}px</span>
            </div>
            <button
              class="reset-btn"
              onclick={() => onprefschange({ sidebarWidth: 220 })}
            >Reset to default</button>
          </div>
        </div>

      {:else if activeSection === 'lsp'}
        <h2 class="section-title">Language Servers</h2>

        <div class="pref-group">
          <div class="pref-row-stacked">
            <span class="pref-name">PHP language server</span>
            <span class="pref-desc">Intelephense (free tier active)</span>
          </div>

          <div class="pref-row-stacked" style="margin-top: 16px;">
            <span class="pref-name">Intelephense License Key</span>
            <span class="pref-desc">Add your key to unlock advanced features (licence.txt from intelephense.com)</span>
            <div class="license-input-row">
              <input
                type={showLicenseKey ? 'text' : 'password'}
                class="text-input"
                placeholder="Not configured — paste your license key here"
                value={intelephenseLicenseKey}
                oninput={(e) => onprefschange({ intelephenseLicenseKey: (e.target as HTMLInputElement).value })}
              />
              <button
                class="show-hide-btn"
                onclick={() => (showLicenseKey = !showLicenseKey)}
                title={showLicenseKey ? 'Hide key' : 'Show key'}
              >{showLicenseKey ? 'Hide' : 'Show'}</button>
            </div>
          </div>
        </div>

        <div class="pref-group">
          <div class="pref-row-stacked">
            <span class="pref-name">JS/TS language server</span>
            <span class="pref-desc">typescript-language-server</span>
          </div>
        </div>

      {:else if activeSection === 'about'}
        <div class="about-section">
          <div class="about-name">Wren</div>
          <div class="about-version">Version 0.2.0</div>
          <p class="about-desc">
            A lightweight editor for PHP &amp; JavaScript, built for Drupal developers.
          </p>
          <p class="about-built">Built with Tauri + CodeMirror 6</p>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    backdrop-filter: blur(2px);
  }

  .modal {
    display: flex;
    width: 680px;
    height: 460px;
    background: #1e1e2e;
    border: 1px solid #313244;
    border-radius: 10px;
    overflow: hidden;
    box-shadow: 0 24px 64px rgba(0, 0, 0, 0.6);
  }

  /* Left nav */
  .prefs-nav {
    width: 160px;
    background: #181825;
    border-right: 1px solid #313244;
    display: flex;
    flex-direction: column;
    padding: 16px 0;
    flex-shrink: 0;
  }

  .prefs-nav-title {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: #585b70;
    padding: 0 14px 12px;
    user-select: none;
  }

  .nav-item {
    all: unset;
    cursor: pointer;
    padding: 7px 14px;
    font-size: 13px;
    color: #7f849c;
    border-radius: 0;
    transition: background 0.1s, color 0.1s;
  }

  .nav-item:hover { color: #cdd6f4; background: #ffffff08; }
  .nav-item.active { color: #cdd6f4; background: #ffffff12; }

  /* Content */
  .prefs-content {
    flex: 1;
    padding: 24px 28px;
    overflow-y: auto;
    position: relative;
    scrollbar-width: thin;
    scrollbar-color: #444 transparent;
  }

  .close-btn {
    all: unset;
    cursor: pointer;
    position: absolute;
    top: 14px;
    right: 16px;
    font-size: 13px;
    color: #585b70;
    padding: 4px 6px;
    border-radius: 4px;
    transition: color 0.1s, background 0.1s;
  }
  .close-btn:hover { color: #cdd6f4; background: #ffffff10; }

  .section-title {
    font-size: 15px;
    font-weight: 600;
    color: #cdd6f4;
    margin: 0 0 20px;
  }

  .pref-group {
    border: 1px solid #313244;
    border-radius: 6px;
    overflow: hidden;
    margin-bottom: 12px;
  }

  .pref-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 12px 14px;
    border-bottom: 1px solid #313244;
  }
  .pref-row:last-child { border-bottom: none; }

  .pref-row-indent {
    background: #181825;
    padding-left: 28px;
  }

  .pref-row-stacked {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 12px 14px;
    border-bottom: 1px solid #313244;
  }
  .pref-row-stacked:last-child { border-bottom: none; }

  .pref-label {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .pref-name {
    font-size: 13px;
    color: #cdd6f4;
  }

  .pref-desc {
    font-size: 11px;
    color: #6c7086;
  }

  /* Toggle switch */
  .toggle {
    all: unset;
    cursor: pointer;
    width: 36px;
    height: 20px;
    background: #313244;
    border-radius: 10px;
    position: relative;
    flex-shrink: 0;
    transition: background 0.2s;
  }
  .toggle.on { background: #89b4fa; }

  .toggle-thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    background: #cdd6f4;
    border-radius: 50%;
    transition: transform 0.2s;
  }
  .toggle.on .toggle-thumb { transform: translateX(16px); }

  /* Slider */
  .slider {
    width: 120px;
    accent-color: #89b4fa;
    flex-shrink: 0;
  }

  /* Theme buttons */
  .theme-options {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .theme-btn {
    all: unset;
    cursor: pointer;
    padding: 4px 10px;
    font-size: 12px;
    color: #7f849c;
    border: 1px solid #313244;
    border-radius: 4px;
    white-space: nowrap;
    transition: background 0.1s, color 0.1s, border-color 0.1s;
  }
  .theme-btn:hover { color: #cdd6f4; background: #ffffff10; }
  .theme-btn.active { color: #89b4fa; border-color: #89b4fa; background: #89b4fa14; }

  /* Font size buttons */
  .font-size-options {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }

  .size-btn {
    all: unset;
    cursor: pointer;
    width: 30px;
    height: 26px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    color: #7f849c;
    border: 1px solid #313244;
    border-radius: 4px;
    transition: background 0.1s, color 0.1s, border-color 0.1s;
  }
  .size-btn:hover { color: #cdd6f4; background: #ffffff10; }
  .size-btn.active { color: #89b4fa; border-color: #89b4fa; background: #89b4fa14; }

  /* Reset button */
  .reset-btn {
    all: unset;
    cursor: pointer;
    font-size: 12px;
    color: #89b4fa;
    padding: 4px 8px;
    border: 1px solid #89b4fa44;
    border-radius: 4px;
    flex-shrink: 0;
    transition: background 0.1s;
  }
  .reset-btn:hover { background: #89b4fa14; }

  /* License input */
  .license-input-row {
    display: flex;
    gap: 6px;
    margin-top: 8px;
  }

  .text-input {
    flex: 1;
    background: #11111b;
    border: 1px solid #313244;
    border-radius: 4px;
    color: #cdd6f4;
    font-size: 12px;
    font-family: monospace;
    padding: 6px 10px;
    outline: none;
    transition: border-color 0.15s;
  }
  .text-input:focus { border-color: #89b4fa; }
  .text-input::placeholder { color: #45475a; }

  .show-hide-btn {
    all: unset;
    cursor: pointer;
    font-size: 12px;
    color: #7f849c;
    padding: 6px 10px;
    border: 1px solid #313244;
    border-radius: 4px;
    white-space: nowrap;
    transition: color 0.1s, background 0.1s;
  }
  .show-hide-btn:hover { color: #cdd6f4; background: #ffffff0a; }

  /* About section */
  .about-section {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    padding-top: 8px;
    gap: 8px;
  }

  .about-name {
    font-size: 28px;
    font-weight: 700;
    color: #cdd6f4;
    letter-spacing: -0.5px;
  }

  .about-version {
    font-size: 12px;
    color: #585b70;
  }

  .about-desc {
    font-size: 13px;
    color: #a6adc8;
    line-height: 1.5;
    margin: 8px 0 0;
    max-width: 380px;
  }

  .about-built {
    font-size: 11px;
    color: #585b70;
    margin: 0;
  }
</style>
