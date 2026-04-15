<script lang="ts">
  import Tooltip from './Tooltip.svelte';
  import { confidenceStore } from '../stores/confidenceStore';

  /**
   * QuickConfidenceFilter - Compact confidence filter toggle for control bar
   *
   * Features:
   * - Quick filter level cycling
   * - Visual status indicator
   * - Keyboard shortcut support
   * - Compact design for control bar integration
   */

  let {
    disabled = false
  }: {
    disabled?: boolean;
  } = $props();

  let settings = $state(confidenceStore.get());
  let dropdownOpen = $state(false);

  // Subscribe to settings changes
  $effect(() => {
    const unsubscribe = confidenceStore.subscribe((newSettings) => {
      settings = newSettings;
    });
    return unsubscribe;
  });

  // Filter level icons and labels
  const filterConfig = {
    none: { icon: '🔓', label: 'Show All', color: 'var(--text-dim)' },
    low: { icon: '🟡', label: 'Low Only', color: 'var(--warning)' },
    medium: { icon: '🟠', label: 'Medium+', color: 'var(--warning)' },
    high: { icon: '🔒', label: 'High Only', color: 'var(--success)' }
  };

  const currentConfig = $derived(filterConfig[settings.filterLevel]);

  // Cycle through filter levels
  function cycleFilter() {
    const levels: Array<'none' | 'low' | 'medium' | 'high'> = ['none', 'low', 'medium', 'high'];
    const currentIndex = levels.indexOf(settings.filterLevel);
    const nextIndex = (currentIndex + 1) % levels.length;
    confidenceStore.update('filterLevel', levels[nextIndex]);
  }

  // Set specific filter level
  function setFilter(level: 'none' | 'low' | 'medium' | 'high') {
    confidenceStore.update('filterLevel', level);
    dropdownOpen = false;
  }

  // Toggle dropdown
  function toggleDropdown() {
    dropdownOpen = !dropdownOpen;
  }

  // Keyboard shortcut handler
  function handleKeydown(e: KeyboardEvent) {
    switch (e.key) {
      case 'Enter':
      case ' ':
        e.preventDefault();
        cycleFilter();
        break;
      case 'ArrowDown':
        e.preventDefault();
        dropdownOpen = true;
        break;
      case 'Escape':
        dropdownOpen = false;
        break;
    }
  }

  // Close dropdown when clicking outside
  function handleClickOutside(e: MouseEvent) {
    if (!e.target) return;
    const target = e.target as HTMLElement;
    if (!target.closest('.quick-confidence-filter')) {
      dropdownOpen = false;
    }
  }
</script>

<svelte:window onclick={handleClickOutside} />

<div class="quick-confidence-filter" class:disabled={disabled}>
  <button
    class="filter-trigger"
    onclick={cycleFilter}
    onkeydown={handleKeydown}
    disabled={disabled}
    aria-label="Confidence filter: {currentConfig.label}"
    aria-haspopup="true"
    aria-expanded={dropdownOpen}
  >
    <span class="filter-icon" style="color: {currentConfig.color}">
      {currentConfig.icon}
    </span>
    <span class="filter-indicator"></span>
  </button>

  {#if dropdownOpen}
    <div class="filter-dropdown" role="menu" aria-label="Confidence filter options">
      {#each ['none', 'low', 'medium', 'high'] as level}
        {@const config = filterConfig[level as keyof typeof filterConfig]}
        {@const isActive = settings.filterLevel === level}
        <button
          class="filter-option"
          class:active={isActive}
          onclick={() => setFilter(level as any)}
          disabled={disabled}
          role="menuitem"
          aria-pressed={isActive}
        >
          <span class="option-icon" style="color: {config.color}">{config.icon}</span>
          <span class="option-label">{config.label}</span>
          {#if isActive}
            <svg class="option-check" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <polyline points="20 6 9 17 4 12"/>
            </svg>
          {/if}
        </button>
      {/each}

      <div class="filter-divider"></div>

      <button
        class="filter-option toggle-option"
        onclick={() => {
          confidenceStore.update('showConfidenceScores', !settings.showConfidenceScores);
        }}
        disabled={disabled}
        role="menuitem"
        aria-pressed={settings.showConfidenceScores}
      >
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z"/>
        </svg>
        <span class="option-label">Show scores</span>
        <div class="toggle-switch" class:active={settings.showConfidenceScores}>
          <span class="toggle-thumb"></span>
        </div>
      </button>
    </div>
  {/if}
</div>

<style>
  .quick-confidence-filter {
    position: relative;
    display: inline-flex;
    align-items: center;
  }

  .quick-confidence-filter.disabled {
    opacity: 0.5;
    pointer-events: none;
  }

  .filter-trigger {
    position: relative;
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .filter-trigger:hover {
    background: var(--bg-hover);
  }

  .filter-trigger:disabled {
    cursor: default;
  }

  .filter-icon {
    font-size: 16px;
    display: block;
    transition: transform var(--transition-fast);
  }

  .filter-trigger:hover .filter-icon {
    transform: scale(1.1);
  }

  .filter-indicator {
    position: absolute;
    bottom: 2px;
    right: 2px;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
    border: 2px solid var(--bg-primary);
    opacity: 0;
    transition: opacity var(--transition-fast);
  }

  .filter-indicator.visible {
    opacity: 1;
  }

  /* Dropdown */
  .filter-dropdown {
    position: absolute;
    bottom: calc(100% + 8px);
    right: 0;
    min-width: 180px;
    background: var(--bg-solid);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-lg);
    padding: var(--space-xs);
    z-index: var(--z-dropdown);
    animation: dropdownSlideIn 0.15s ease;
  }

  @keyframes dropdownSlideIn {
    from {
      opacity: 0;
      transform: translateY(8px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .filter-option {
    width: 100%;
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-sm) var(--space-md);
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    font-size: var(--font-size-sm);
    font-family: var(--font-family);
    cursor: pointer;
    transition: all var(--transition-fast);
    text-align: left;
  }

  .filter-option:hover {
    background: var(--bg-hover);
  }

  .filter-option.active {
    background: var(--accent-dim);
  }

  .filter-option:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .option-icon {
    font-size: 16px;
    flex-shrink: 0;
  }

  .option-label {
    flex: 1;
    font-size: var(--font-size-sm);
  }

  .option-check {
    width: 14px;
    height: 14px;
    color: var(--accent);
    flex-shrink: 0;
  }

  .filter-divider {
    height: 1px;
    background: var(--border);
    margin: var(--space-xs) 0;
  }

  .toggle-option {
    justify-content: space-between;
  }

  .toggle-switch {
    width: 32px;
    height: 18px;
    background: rgba(255, 255, 255, 0.1);
    border-radius: 9px;
    position: relative;
    transition: background var(--transition-fast);
    flex-shrink: 0;
  }

  .toggle-switch.active {
    background: var(--accent);
  }

  .toggle-thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 14px;
    height: 14px;
    background: white;
    border-radius: 50%;
    transition: transform var(--transition-fast);
  }

  .toggle-switch.active .toggle-thumb {
    transform: translateX(14px);
  }

  /* Accessibility */
  @media (prefers-reduced-motion: reduce) {
    .filter-dropdown,
    .filter-trigger,
    .filter-option,
    .filter-icon,
    .toggle-switch,
    .toggle-thumb {
      animation: none;
      transition: none;
    }
  }
</style>