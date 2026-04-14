<!--
  Modal Component

  Reusable modal with backdrop, header, and content slots.
  Used by QuickSelector components and other overlays.

  Props:
    - show: boolean - Whether to show the modal
    - title: string - Modal title
    - onClose: () => void - Callback when modal should close
    - closeOnBackdropClick: boolean - Whether clicking backdrop closes modal (default: true)
    - closeOnEscape: boolean - Whether Escape key closes modal (default: true)
-->

<script lang="ts">
  let {
    show = false,
    title = '',
    onClose,
    closeOnBackdropClick = true,
    closeOnEscape = true
  }: {
    show: boolean;
    title: string;
    onClose?: () => void;
    closeOnBackdropClick?: boolean;
    closeOnEscape?: boolean;
  } = $props();

  // Handle backdrop click
  function handleBackdropClick(e: MouseEvent) {
    if (closeOnBackdropClick && e.target === e.currentTarget) {
      onClose?.();
    }
  }

  // Handle Escape key
  function handleKeydown(e: KeyboardEvent) {
    if (closeOnEscape && e.key === 'Escape' && show) {
      onClose?.();
    }
  }

  // Set up/clean up keyboard listener
  $effect(() => {
    if (show && closeOnEscape) {
      window.addEventListener('keydown', handleKeydown);
      return () => window.removeEventListener('keydown', handleKeydown);
    }
  });
</script>

{#if show}
  <div
    class="modal-backdrop"
    onclick={handleBackdropClick}
    role="presentation"
  >
    <div
      class="modal-panel"
      role="dialog"
      aria-modal="true"
      aria-labelledby="modal-title"
    >
      <div class="modal-header">
        <h2 id="modal-title" class="modal-title">{title}</h2>
        <button
          class="modal-close"
          onclick={onClose}
          aria-label="Close modal"
          type="button"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="18" y1="6" x2="6" y2="18"/>
            <line x1="6" y1="6" x2="18" y2="18"/>
          </svg>
        </button>
      </div>

      <div class="modal-content">
        <slot />
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(4px);
    z-index: var(--z-overlay);
    display: flex;
    align-items: center;
    justify-content: center;
    animation: fadeIn 0.15s ease;
    padding: 20px;
  }

  .modal-panel {
    background: var(--bg-solid);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
    max-width: 480px;
    width: 100%;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    animation: slideUp 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .modal-title {
    font-size: var(--font-size-md);
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .modal-close {
    width: 32px;
    height: 32px;
    border-radius: var(--radius-sm);
    border: none;
    background: transparent;
    color: var(--text-dim);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s ease;
    flex-shrink: 0;
    padding: 0;
  }

  .modal-close:hover {
    color: var(--text-primary);
    background: var(--bg-hover);
  }

  .modal-close svg {
    width: 18px;
    height: 18px;
  }

  .modal-content {
    padding: 20px;
    overflow-y: auto;
    flex: 1;
  }

  /* Scrollbar styling for modal content */
  .modal-content::-webkit-scrollbar {
    width: 8px;
  }

  .modal-content::-webkit-scrollbar-track {
    background: transparent;
  }

  .modal-content::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.1);
    border-radius: 4px;
  }

  .modal-content::-webkit-scrollbar-thumb:hover {
    background: rgba(255, 255, 255, 0.2);
  }
</style>
