<!--
  Unified Button Component

  Standardized button with multiple variants to replace inconsistent
  button implementations throughout the application.

  Props:
    - variant: 'primary' | 'secondary' | 'ghost' | 'danger' | 'icon'
    - size: 'sm' | 'md' | 'lg'
    - icon: Component | null - Icon component to display
    - iconPosition: 'left' | 'right' - Position of icon (default: 'left')
    - loading: boolean - Show loading state
    - disabled: boolean - Disable the button
    - onclick: () => void - Click handler
    - type: 'button' | 'submit' | 'reset' - HTML button type
    - title: string - Tooltip text
    - fullWidth: boolean - Whether button should take full width
-->

<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    variant = 'secondary',
    size = 'md',
    loading = false,
    disabled = false,
    onclick,
    type = 'button',
    title = '',
    fullWidth = false,
    children
  }: {
    variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
    size?: 'sm' | 'md' | 'lg';
    loading?: boolean;
    disabled?: boolean;
    onclick?: () => void;
    type?: 'button' | 'submit' | 'reset';
    title?: string;
    fullWidth?: boolean;
    children?: Snippet;
  } = $props();

  function handleClick(e: MouseEvent) {
    if (disabled || loading) {
      e.preventDefault();
      return;
    }
    onclick?.();
  }
</script>

<button
  class="btn"
  class:variant={variant}
  class:size={size}
  class:loading={loading}
  class:disabled={disabled || loading}
  class:full-width={fullWidth}
  {type}
  {title}
  onclick={handleClick}
>
  {#if loading}
    <span class="btn-spinner" aria-hidden="true"></span>
  {:else}
    {@render children?.()}
  {/if}
</button>

<style>
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    border: none;
    border-radius: var(--radius-sm);
    font-family: var(--font-family);
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    white-space: nowrap;
    position: relative;
    overflow: hidden;
  }

  /* Size variants */
  .btn.sm {
    height: 32px;
    padding: 0 12px;
    font-size: var(--font-size-xs);
    gap: 6px;
  }

  .btn.md {
    height: 38px;
    padding: 0 16px;
    font-size: var(--font-size-sm);
    gap: 8px;
  }

  .btn.lg {
    height: 44px;
    padding: 0 20px;
    font-size: var(--font-size-md);
    gap: 10px;
  }

  /* Icon-only buttons */
  .btn.with-icon.icon {
    width: 32px;
    height: 32px;
    padding: 0;
  }

  .btn.sm.icon {
    width: 28px;
    height: 28px;
    padding: 0;
  }

  .btn.lg.icon {
    width: 40px;
    height: 40px;
    padding: 0;
  }

  /* Primary variant */
  .btn.primary {
    background: linear-gradient(135deg, var(--accent) 0%, #5a7fd4 100%);
    color: white;
    border: 1px solid rgba(255, 255, 255, 0.1);
    box-shadow: 0 2px 8px rgba(99, 140, 255, 0.4);
  }

  .btn.primary:hover:not(.disabled) {
    background: linear-gradient(135deg, var(--accent-hover) 0%, #6b8ae8 100%);
    box-shadow: 0 3px 12px rgba(99, 140, 255, 0.5);
    transform: translateY(-1px);
  }

  .btn.primary:active:not(.disabled) {
    transform: translateY(0);
  }

  /* Secondary variant */
  .btn.secondary {
    background: var(--bg-secondary);
    color: var(--text-secondary);
    border: 1px solid var(--border);
  }

  .btn.secondary:hover:not(.disabled) {
    background: var(--bg-hover);
    border-color: var(--border-hover);
    color: var(--text-primary);
  }

  /* Ghost variant */
  .btn.ghost {
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid transparent;
  }

  .btn.ghost:hover:not(.disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  /* Danger variant */
  .btn.danger {
    background: rgba(255, 77, 77, 0.1);
    color: var(--danger);
    border: 1px solid rgba(255, 77, 77, 0.2);
  }

  .btn.danger:hover:not(.disabled) {
    background: rgba(255, 77, 77, 0.2);
    border-color: rgba(255, 77, 77, 0.3);
    color: var(--danger-hover);
  }

  /* Icon variant */
  .btn.icon {
    padding: 0;
    background: transparent;
    border: none;
    color: var(--text-dim);
    box-shadow: none;
  }

  .btn.icon:hover:not(.disabled) {
    background: var(--bg-hover);
    color: var(--text-secondary);
  }

  /* Full width */
  .btn.full-width {
    width: 100%;
  }

  /* Disabled state */
  .btn.disabled {
    opacity: 0.5;
    cursor: not-allowed;
    pointer-events: none;
  }

  /* Icon styling */
  .btn-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .btn-icon svg {
    width: 16px;
    height: 16px;
  }

  .btn.sm .btn-icon svg,
  .btn.icon .btn-icon svg {
    width: 14px;
    height: 14px;
  }

  .btn.lg .btn-icon svg {
    width: 18px;
    height: 18px;
  }

  .btn-icon-left {
    margin-right: -4px;
  }

  .btn-icon-right {
    margin-left: -4px;
  }

  /* Content */
  .btn-content {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  /* Loading spinner */
  .btn-spinner {
    width: 14px;
    height: 14px;
    border: 2px solid transparent;
    border-top-color: currentColor;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  .btn.sm .btn-spinner {
    width: 12px;
    height: 12px;
    border-width: 1.5px;
  }

  .btn.lg .btn-spinner {
    width: 16px;
    height: 16px;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
