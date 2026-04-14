<script lang="ts">
  let {
    show = false,
    onClose
  }: {
    show: boolean;
    onClose: () => void;
  } = $props();

  const shortcuts = [
    {
      category: 'Recording',
      items: [
        { key: 'Space', description: 'Start / Stop recording' }
      ]
    },
    {
      category: 'Navigation',
      items: [
        { key: '⌘ ,', description: 'Open Settings' },
        { key: '⌘ H', description: 'View saved transcripts' }
      ]
    },
    {
      category: 'Editing',
      items: [
        { key: '⌘ K', description: 'Clear transcript' }
      ]
    }
  ];

  function handleBackdropClick(e: MouseEvent) {
    if ((e.target as HTMLElement).classList.contains('shortcuts-backdrop')) {
      onClose();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    }
  }
</script>

{#if show}
  <div class="shortcuts-backdrop" onclick={handleBackdropClick} onkeydown={handleKeydown}>
    <div class="shortcuts-panel">
      <div class="shortcuts-header">
        <h2>Keyboard Shortcuts</h2>
        <button class="btn-close" onclick={onClose} aria-label="Close">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="18" y1="6" x2="6" y2="18"/>
            <line x1="6" y1="6" x2="18" y2="18"/>
          </svg>
        </button>
      </div>

      <div class="shortcuts-content">
        {#each shortcuts as category}
          <div class="shortcut-category">
            <h3 class="category-title">{category.category}</h3>
            <div class="shortcut-list">
              {#each category.items as item}
                <div class="shortcut-item">
                  <span class="shortcut-description">{item.description}</span>
                  <kbd class="shortcut-key">{item.key}</kbd>
                </div>
              {/each}
            </div>
          </div>
        {/each}

        <div class="shortcuts-footer">
          <p class="shortcuts-hint">Press <kbd>Escape</kbd> to close</p>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .shortcuts-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: var(--z-overlay);
    animation: fadeIn 0.2s ease;
  }

  .shortcuts-panel {
    width: 90%;
    max-width: 480px;
    max-height: 80vh;
    background: var(--bg-solid);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
    animation: slideUp 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    overflow: hidden;
  }

  .shortcuts-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-lg) var(--space-xl);
    border-bottom: 1px solid var(--border);
  }

  .shortcuts-header h2 {
    font-size: var(--font-size-lg);
    font-weight: 700;
    color: var(--text-primary);
    margin: 0;
  }

  .btn-close {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-dim);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .btn-close:hover {
    background: rgba(255, 255, 255, 0.08);
    color: var(--text-primary);
  }

  .shortcuts-content {
    padding: var(--space-lg) var(--space-xl);
    overflow-y: auto;
    flex: 1;
  }

  .shortcut-category {
    margin-bottom: var(--space-lg);
  }

  .shortcut-category:last-child {
    margin-bottom: 0;
  }

  .category-title {
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin: 0 0 var(--space-sm) 0;
  }

  .shortcut-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .shortcut-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-sm) var(--space-md);
    background: rgba(255, 255, 255, 0.02);
    border-radius: var(--radius-md);
    transition: background 0.2s ease;
  }

  .shortcut-item:hover {
    background: rgba(255, 255, 255, 0.04);
  }

  .shortcut-description {
    font-size: var(--font-size-md);
    color: var(--text-primary);
  }

  .shortcut-key {
    display: inline-flex;
    align-items: center;
    padding: 4px 10px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-sm);
    font-weight: 600;
    font-family: var(--font-family);
    color: var(--text-primary);
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
  }

  .shortcuts-footer {
    margin-top: var(--space-lg);
    padding-top: var(--space-md);
    border-top: 1px solid var(--border);
    text-align: center;
  }

  .shortcuts-hint {
    font-size: var(--font-size-sm);
    color: var(--text-dim);
    margin: 0;
  }

  .shortcuts-hint kbd {
    margin: 0 4px;
  }
</style>
