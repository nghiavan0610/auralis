<script lang="ts">
  import Modal from './Modal.svelte';

  let {
    show = false,
    currentProvider = 'webspeech',
    hasApiKey = { google: false, elevenlabs: false },
    onSelect,
    onClose
  }: {
    show: boolean;
    currentProvider?: 'webspeech' | 'edge' | 'google' | 'elevenlabs';
    hasApiKey?: { google: boolean; elevenlabs: boolean };
    onSelect: (provider: 'webspeech' | 'edge' | 'google' | 'elevenlabs', needsSettings: boolean) => void;
    onClose?: () => void;
  } = $props();

  let localProvider = $state(currentProvider);

  const providers = [
    {
      id: 'webspeech' as const,
      name: 'Web Speech',
      description: 'Built-in browser voices, free and always available',
      icon: '🌐',
      features: ['Free', 'No setup', 'Limited voices'],
      needsKey: false
    },
    {
      id: 'edge' as const,
      name: 'Edge TTS',
      description: 'Microsoft Edge natural voices, high quality',
      icon: '🔊',
      features: ['Free', 'High quality', 'Many voices'],
      needsKey: false
    },
    {
      id: 'google' as const,
      name: 'Google TTS',
      description: 'Google Cloud Text-to-Speech with premium voices',
      icon: '🗣️',
      features: ['Premium', 'API key required', 'Natural voices'],
      needsKey: true,
      keyName: 'google'
    },
    {
      id: 'elevenlabs' as const,
      name: 'ElevenLabs',
      description: 'AI-powered voices with realistic intonation',
      icon: '✨',
      features: ['Premium', 'API key required', 'Best quality'],
      needsKey: true,
      keyName: 'elevenlabs'
    }
  ];

  function handleSelect(provider: typeof providers[number]) {
    const needsSettings = provider.needsKey && !hasApiKey[provider.keyName!];
    onSelect(provider.id, needsSettings);
    if (!needsSettings) {
      close();
    }
  }

  function close() {
    localProvider = currentProvider;
    onClose?.();
  }
</script>

<Modal show={show} title="Select TTS Provider" onClose={close}>
  <div class="provider-list">
    {#each providers as provider}
      <button
        class="provider-card"
        class:selected={localProvider === provider.id}
        onclick={() => handleSelect(provider)}
      >
        <div class="provider-card-header">
          <div class="provider-icon">{provider.icon}</div>
          <div class="provider-info">
            <div class="provider-name">{provider.name}</div>
            <div class="provider-description">{provider.description}</div>
          </div>
          {#if provider.needsKey && !hasApiKey[provider.keyName!]}
            <div class="provider-badge warning">API Key</div>
          {:else if provider.needsKey}
            <div class="provider-badge success">Ready</div>
          {:else}
            <div class="provider-badge free">Free</div>
          {/if}
        </div>
        <div class="provider-features">
          {#each provider.features as feature}
            <div class="feature-tag">{feature}</div>
          {/each}
        </div>
        {#if provider.needsKey && !hasApiKey[provider.keyName!]}
          <div class="provider-warning">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="var(--warning)" stroke-width="2">
              <circle cx="12" cy="12" r="10"/>
              <line x1="12" y1="8" x2="12" y2="12"/>
              <line x1="12" y1="16" x2="12.01" y2="16"/>
            </svg>
            <span>You'll be redirected to settings to add your API key</span>
          </div>
        {/if}
      </button>
    {/each}
  </div>
</Modal>

<style>
  .provider-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .provider-card {
    background: var(--bg-secondary);
    border: 2px solid var(--border);
    border-radius: var(--radius-md);
    padding: var(--space-md);
    cursor: pointer;
    transition: all 0.2s ease;
    text-align: left;
  }

  .provider-card:hover {
    background: var(--bg-hover);
    border-color: var(--border-hover);
  }

  .provider-card.selected {
    background: rgba(99, 140, 255, 0.08);
    border-color: var(--accent);
  }

  .provider-card-header {
    display: flex;
    align-items: center;
    gap: var(--space-md);
    margin-bottom: var(--space-sm);
  }

  .provider-icon {
    width: 36px;
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 20px;
    background: rgba(99, 140, 255, 0.1);
    border-radius: var(--radius-sm);
  }

  .provider-info {
    flex: 1;
  }

  .provider-name {
    font-size: var(--font-size-md);
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 2px;
  }

  .provider-description {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    line-height: 1.3;
  }

  .provider-badge {
    padding: 2px 8px;
    border-radius: var(--radius-sm);
    font-size: var(--font-size-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .provider-badge.warning {
    background: rgba(255, 165, 0, 0.15);
    color: var(--warning);
  }

  .provider-badge.success {
    background: rgba(0, 200, 83, 0.15);
    color: var(--success);
  }

  .provider-badge.free {
    background: rgba(99, 140, 255, 0.15);
    color: var(--accent);
  }

  .provider-features {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-xs);
  }

  .feature-tag {
    padding: 2px 8px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-xs);
    color: var(--text-dim);
  }

  .provider-warning {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    margin-top: var(--space-sm);
    padding: var(--space-xs) var(--space-sm);
    background: rgba(255, 165, 0, 0.1);
    border: 1px solid rgba(255, 165, 0, 0.2);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-xs);
    color: var(--warning);
  }
</style>
