<script lang="ts">
  let expanded = $state(false);
</script>

<div class="setup-guide">
  <h3 onclick={() => expanded = !expanded} class="toggle-header">
    Setup Guide {expanded ? '▾' : '▸'}
  </h3>

  {#if expanded}
    <div class="guide-content">
      <div class="mode-info">
        <h4>Cloud Mode (Soniox)</h4>
        <p>No downloads required. Real-time STT + translation via Soniox WebSocket API (~150-300ms latency).</p>
        <ol>
          <li>Get a free API key at <a href="https://soniox.com/" target="_blank" class="link">soniox.com</a></li>
          <li>Paste the key in the settings above</li>
          <li>Click Start</li>
        </ol>
      </div>

      <div class="mode-info">
        <h4>Offline Mode (MLX)</h4>
        <p>Runs locally with MLX Whisper + Opus-MT. Models are auto-downloaded on first use (~3 GB).</p>
        <ol>
          <li>Set up the Python environment:</li>
          <div class="code-block">
            <code>python3 -m venv ~/.config/auralis/mlx-env</code><br>
            <code>source ~/.config/auralis/mlx-env/bin/activate</code><br>
            <code>pip install mlx-whisper transformers numpy</code>
          </div>
          <li>Select "Offline" mode above</li>
          <li>Click Start — models download automatically on first run</li>
        </ol>
      </div>
    </div>
  {/if}
</div>

<style>
  .setup-guide {
    margin-top: 1rem;
    padding: 1rem 1.5rem;
    background-color: rgba(255, 255, 255, 0.03);
    border-radius: 8px;
  }

  .toggle-header {
    font-size: 1rem;
    cursor: pointer;
    color: rgba(255, 255, 255, 0.7);
    margin: 0;
    user-select: none;
  }

  .toggle-header:hover {
    color: rgba(255, 255, 255, 0.9);
  }

  .guide-content {
    margin-top: 1rem;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }

  .mode-info {
    padding: 1rem;
    background-color: rgba(255, 255, 255, 0.03);
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.08);
  }

  .mode-info h4 {
    margin: 0 0 0.5rem 0;
    font-size: 0.95rem;
    color: rgba(255, 255, 255, 0.9);
  }

  .mode-info p {
    margin: 0 0 0.75rem 0;
    font-size: 0.85rem;
    color: rgba(255, 255, 255, 0.6);
  }

  .mode-info ol {
    margin: 0;
    padding-left: 1.25rem;
    font-size: 0.85rem;
    color: rgba(255, 255, 255, 0.7);
  }

  .mode-info ol li {
    margin-bottom: 0.5rem;
  }

  .code-block {
    margin: 0.5rem 0 0.75rem;
    padding: 0.75rem;
    background-color: rgba(0, 0, 0, 0.3);
    border-radius: 4px;
    font-family: 'Menlo', 'Monaco', 'Courier New', monospace;
    font-size: 0.8rem;
    color: #a5d6ff;
    line-height: 1.6;
  }

  .link {
    color: #646cff;
    text-decoration: none;
  }

  .link:hover {
    text-decoration: underline;
  }
</style>
