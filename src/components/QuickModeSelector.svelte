<script lang="ts">
  import Modal from './Modal.svelte';

  let {
    show = false,
    currentMode = 'cloud',
    hasApiKey = true,
    onSelect,
    onClose
  }: {
    show: boolean;
    currentMode?: 'cloud' | 'offline';
    hasApiKey?: boolean;
    onSelect: (mode: 'cloud' | 'offline', needsSettings: boolean) => void;
    onClose?: () => void;
  } = $props();

  let localMode = $state(currentMode);

  function handleSelect(mode: 'cloud' | 'offline') {
    const needsSettings = mode === 'cloud' && !hasApiKey;
    onSelect(mode, needsSettings);
    if (!needsSettings) {
      close();
    }
  }

  function close() {
    localMode = currentMode;
    onClose?.();
  }
</script>

<Modal show={show} title="Select Translation Mode" onClose={close}>
  <div class="mode-options">
    <button
      class="mode-option"
      class:selected={localMode === 'cloud'}
      onclick={() => handleSelect('cloud')}
    >
      <div class="mode-option-content">
        <div class="mode-option-header">
          <div class="mode-option-icon">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M18 10h-1.26A8 8 0 1 0 9 20h9a5 5 0 0 0 0-10z"/>
            </svg>
          </div>
          <div class="mode-option-info">
            <div class="mode-option-title">Cloud Mode</div>
            <div class="mode-option-description">
              Uses Soniox cloud API for high-quality translation
            </div>
          </div>
          {#if !hasApiKey}
            <div class="mode-option-badge warning">API Key Required</div>
          {/if}
        </div>
        <div class="mode-option-features">
          <div class="feature">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--success)" stroke-width="2">
              <polyline points="20 6 9 17 4 12"/>
            </svg>
            <span>High accuracy</span>
          </div>
          <div class="feature">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--success)" stroke-width="2">
              <polyline points="20 6 9 17 4 12"/>
            </svg>
            <span>Fast response</span>
          </div>
          <div class="feature">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--success)" stroke-width="2">
              <polyline points="20 6 9 17 4 12"/>
            </svg>
            <span>No setup needed</span>
          </div>
        </div>
        {#if !hasApiKey}
          <div class="mode-option-warning">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--warning)" stroke-width="2">
              <circle cx="12" cy="12" r="10"/>
              <line x1="12" y1="8" x2="12" y2="12"/>
              <line x1="12" y1="16" x2="12.01" y2="16"/>
            </svg>
            <span>You'll be redirected to settings to add your API key</span>
          </div>
        {/if}
      </div>
    </button>

    <button
      class="mode-option"
      class:selected={localMode === 'offline'}
      onclick={() => handleSelect('offline')}
    >
      <div class="mode-option-content">
        <div class="mode-option-header">
          <div class="mode-option-icon">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <rect x="3" y="3" width="18" height="18" rx="2"/>
            </svg>
          </div>
          <div class="mode-option-info">
            <div class="mode-option-title">Offline Mode</div>
            <div class="mode-option-description">
              Runs locally with MLX models - requires initial download
            </div>
          </div>
          <div class="mode-option-badge success">Free</div>
        </div>
        <div class="mode-option-features">
          <div class="feature">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--success)" stroke-width="2">
              <polyline points="20 6 9 17 4 12"/>
            </svg>
            <span>100% private</span>
          </div>
          <div class="feature">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--success)" stroke-width="2">
              <polyline points="20 6 9 17 4 12"/>
            </svg>
            <span>No internet needed</span>
          </div>
          <div class="feature">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--success)" stroke-width="2">
              <polyline points="20 6 9 17 4 12"/>
            </svg>
            <span>Unlimited use</span>
          </div>
        </div>
      </div>
    </button>
  </div>
</Modal>

<style>
  .mode-options {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }

  .mode-option {
    background: var(--bg-secondary);
    border: 2px solid var(--border);
    border-radius: var(--radius-lg);
    padding: var(--space-md);
    cursor: pointer;
    transition: all 0.2s ease;
    text-align: left;
  }

  .mode-option:hover {
    background: var(--bg-hover);
    border-color: var(--border-hover);
  }

  .mode-option.selected {
    background: rgba(99, 140, 255, 0.08);
    border-color: var(--accent);
  }

  .mode-option-content {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .mode-option-header {
    display: flex;
    align-items: flex-start;
    gap: var(--space-md);
  }

  .mode-option-icon {
    width: 40px;
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(99, 140, 255, 0.1);
    border-radius: var(--radius-md);
    color: var(--accent);
    flex-shrink: 0;
  }

  .mode-option-info {
    flex: 1;
  }

  .mode-option-title {
    font-size: var(--font-size-md);
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 2px;
  }

  .mode-option-description {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    line-height: 1.4;
  }

  .mode-option-badge {
    padding: 2px 8px;
    border-radius: var(--radius-sm);
    font-size: var(--font-size-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .mode-option-badge.warning {
    background: rgba(255, 165, 0, 0.15);
    color: var(--warning);
  }

  .mode-option-badge.success {
    background: rgba(0, 200, 83, 0.15);
    color: var(--success);
  }

  .mode-option-features {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-sm);
  }

  .feature {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
  }

  .mode-option-warning {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    padding: var(--space-sm);
    background: rgba(255, 165, 0, 0.1);
    border: 1px solid rgba(255, 165, 0, 0.2);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-xs);
    color: var(--warning);
  }
</style>
