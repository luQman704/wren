<script lang="ts">
  interface Props {
    filename: string;
    onreload: () => void;
    ondismiss: () => void;
  }

  let { filename, onreload, ondismiss }: Props = $props();
</script>

<div class="toast" role="alert">
  <span class="msg">
    <strong>{filename}</strong> was changed on disk
  </span>
  <div class="actions">
    <button class="btn-reload" onclick={onreload}>Reload</button>
    <button class="btn-dismiss" onclick={ondismiss}>Dismiss</button>
  </div>
</div>

<style>
  .toast {
    position: fixed;
    bottom: 32px;
    right: 16px;
    display: flex;
    align-items: center;
    gap: 12px;
    background: #2a2a3c;
    border: 1px solid #444;
    border-radius: 6px;
    padding: 8px 14px;
    font-size: 12px;
    color: #cdd6f4;
    z-index: 9000;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.5);
    animation: slideIn 0.15s ease;
  }

  @keyframes slideIn {
    from { opacity: 0; transform: translateY(8px); }
    to   { opacity: 1; transform: translateY(0); }
  }

  .msg { flex: 1; }
  .msg strong { color: #89b4fa; }

  .actions { display: flex; gap: 6px; }

  button {
    all: unset;
    cursor: pointer;
    padding: 3px 10px;
    border-radius: 4px;
    font-size: 11px;
    border: 1px solid transparent;
  }

  .btn-reload {
    background: #313244;
    color: #cdd6f4;
    border-color: #555;
  }
  .btn-reload:hover { background: #414155; }

  .btn-dismiss {
    color: #888;
  }
  .btn-dismiss:hover { color: #cdd6f4; }
</style>
