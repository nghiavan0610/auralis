<script lang="ts">
  import { onMount } from 'svelte';

  export type TabId = 'translation' | 'display' | 'tts' | 'subscription' | 'about';

  export interface Tab {
    id: TabId;
    label: string;
    icon: string;
  }

  interface Props {
    activeTab: TabId;
    tabs: Tab[];
    onchange: (tabId: TabId) => void;
  }

  let { activeTab, tabs, onchange }: Props = $props();

  let tabElements: Record<TabId, HTMLElement> = $state({} as Record<TabId, HTMLElement>);
  let indicatorPosition = $state<{ left: number; width: number }>({ left: 0, width: 0 });

  function selectTab(tabId: TabId) {
    onchange(tabId);
    updateIndicator();
  }

  function setTabElement(tabId: TabId, el: HTMLElement) {
    tabElements[tabId] = el;
  }

  function updateIndicator() {
    const activeElement = tabElements[activeTab];
    if (activeElement) {
      indicatorPosition = {
        left: activeElement.offsetLeft,
        width: activeElement.offsetWidth
      };
    }
  }

  function tabAction(node: HTMLElement, { tabId, setTabElement }: { tabId: TabId; setTabElement: (tabId: TabId, el: HTMLElement) => void }) {
    setTabElement(tabId, node);
    return {
      destroy() {
        // Cleanup if needed
      }
    };
  }

  onMount(() => {
    // Initial position
    setTimeout(updateIndicator, 0);

    // Recalculate on window resize
    window.addEventListener('resize', updateIndicator);

    return () => {
      window.removeEventListener('resize', updateIndicator);
    };
  });

  // Keyboard navigation
  function handleKeydown(event: KeyboardEvent, tabId: TabId) {
    const currentIndex = tabs.findIndex(tab => tab.id === tabId);

    switch (event.key) {
      case 'ArrowLeft':
        event.preventDefault();
        const prevIndex = currentIndex > 0 ? currentIndex - 1 : tabs.length - 1;
        selectTab(tabs[prevIndex].id);
        tabElements.get(tabs[prevIndex].id)?.focus();
        break;
      case 'ArrowRight':
        event.preventDefault();
        const nextIndex = currentIndex < tabs.length - 1 ? currentIndex + 1 : 0;
        selectTab(tabs[nextIndex].id);
        tabElements.get(tabs[nextIndex].id)?.focus();
        break;
      case 'Home':
        event.preventDefault();
        selectTab(tabs[0].id);
        tabElements.get(tabs[0].id)?.focus();
        break;
      case 'End':
        event.preventDefault();
        selectTab(tabs[tabs.length - 1].id);
        tabElements.get(tabs[tabs.length - 1].id)?.focus();
        break;
    }
  }
</script>

<div class="tab-navigation" role="tablist">
  <div class="tab-indicator" style="transform: translateX({indicatorPosition.left}px); width: {indicatorPosition.width}px;"></div>

  {#each tabs as tab (tab.id)}
    <button
      class="tab-button"
      class:active={activeTab === tab.id}
      role="tab"
      aria-selected={activeTab === tab.id}
      aria-controls="{tab.id}-panel"
      tabindex={activeTab === tab.id ? 0 : -1}
      onclick={() => selectTab(tab.id)}
      onkeydown={(e) => handleKeydown(e, tab.id)}
      use:tabAction={{ tabId: tab.id, setTabElement }}
    >
      <span class="tab-icon" aria-hidden="true">
        {@html tab.icon}
      </span>
      <span class="tab-label">{tab.label}</span>
    </button>
  {/each}
</div>

<style>
  .tab-navigation {
    position: relative;
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 6px;
    background: var(--bg-secondary);
    border-radius: var(--radius-md);
    border: 1px solid var(--border);
    margin-bottom: var(--space-lg);
  }

  .tab-indicator {
    position: absolute;
    top: 6px;
    bottom: 6px;
    height: calc(100% - 12px);
    background: var(--accent);
    border-radius: var(--radius-sm);
    transition: transform 0.25s cubic-bezier(0.4, 0, 0.2, 1),
                width 0.25s cubic-bezier(0.4, 0, 0.2, 1);
    box-shadow: 0 2px 8px rgba(99, 140, 255, 0.4);
    z-index: var(--z-background);
  }

  .tab-button {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 16px;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    font-family: var(--font-family);
    font-size: var(--font-size-sm);
    font-weight: 500;
    cursor: pointer;
    transition: color 0.2s ease;
    z-index: var(--z-base);
    white-space: nowrap;
  }

  .tab-button:hover {
    color: var(--text-primary);
  }

  .tab-button.active {
    color: white;
  }

  .tab-button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .tab-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
  }

  .tab-icon :global(svg) {
    width: 18px;
    height: 18px;
    stroke-width: 2;
  }

  /* Responsive adjustments */
  @media (max-width: 600px) {
    .tab-button {
      padding: 8px 12px;
      font-size: var(--font-size-xs);
    }

    .tab-icon {
      width: 16px;
      height: 16px;
    }

    .tab-icon :global(svg) {
      width: 16px;
      height: 16px;
    }
  }
</style>