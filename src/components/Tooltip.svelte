<script lang="ts">
  let {
    content = '',
    position = 'top',
    delay = 200
  }: {
    content: string;
    position?: 'top' | 'bottom' | 'left' | 'right';
    delay?: number;
  } = $props();

  let visible = $state(false);
  let timeout: ReturnType<typeof setTimeout> | null = null;
  let triggerEl: HTMLElement;
  let tooltipEl: HTMLElement;

  function show() {
    if (timeout) clearTimeout(timeout);
    timeout = setTimeout(() => {
      visible = true;
    }, delay);
  }

  function hide() {
    if (timeout) clearTimeout(timeout);
    visible = false;
  }

  function getPosition() {
    if (!triggerEl || !tooltipEl) return {};

    // Get the first child (the actual button/content) for positioning
    const contentEl = triggerEl.firstElementChild as HTMLElement;
    if (!contentEl) return {};

    const triggerRect = contentEl.getBoundingClientRect();
    const tooltipRect = tooltipEl.getBoundingClientRect();
    const gap = 8;

    switch (position) {
      case 'top':
        return {
          top: triggerRect.top - gap - tooltipRect.height,
          left: triggerRect.left + (triggerRect.width - tooltipRect.width) / 2
        };
      case 'bottom':
        return {
          top: triggerRect.bottom + gap,
          left: triggerRect.left + (triggerRect.width - tooltipRect.width) / 2
        };
      case 'left':
        return {
          top: triggerRect.top + (triggerRect.height - tooltipRect.height) / 2,
          right: window.innerWidth - triggerRect.left + gap
        };
      case 'right':
        return {
          top: triggerRect.top + (triggerRect.height - tooltipRect.height) / 2,
          left: triggerRect.right + gap
        };
      default:
        return {};
    }
  }

  function tooltipAction(node: HTMLElement, { getPosition, tooltipEl }: { getPosition: () => {}, tooltipEl: any }) {
    const updatePosition = () => {
      const pos = getPosition();
      Object.assign(node.style, pos);
    };

    // Update position on next frame
    requestAnimationFrame(updatePosition);

    return {
      update() {
        updatePosition();
      },
      destroy() {}
    };
  }
</script>

<div class="tooltip-wrapper" onmouseenter={show} onmouseleave={hide} bind:this={triggerEl}>
  <slot />

  {#if visible}
    <div
      class="tooltip"
      class:tooltip-top={position === 'top'}
      class:tooltip-bottom={position === 'bottom'}
      class:tooltip-left={position === 'left'}
      class:tooltip-right={position === 'right'}
      use:tooltipAction={{ getPosition, tooltipEl }}
    >
      {content}
    </div>
  {/if}
</div>

<style>
  .tooltip-wrapper {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .tooltip-wrapper > :global(*) {
    cursor: inherit;
  }

  .tooltip {
    position: fixed;
    z-index: var(--z-dropdown);
    max-width: 280px;
    padding: 8px 12px;
    background: rgba(15, 15, 20, 0.95);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    font-size: var(--font-size-xs);
    line-height: 1.4;
    pointer-events: none;
    box-shadow: var(--shadow-md);
    white-space: normal;
  }

  .tooltip::before {
    content: '';
    position: absolute;
    border: 6px solid transparent;
  }

  .tooltip-top::before {
    bottom: 100%;
    left: 50%;
    transform: translateX(-50%);
    border-top-color: var(--border);
  }

  .tooltip-bottom::before {
    top: 100%;
    left: 50%;
    transform: translateX(-50%);
    border-bottom-color: var(--border);
  }

  .tooltip-left::before {
    right: 100%;
    top: 50%;
    transform: translateY(-50%);
    border-left-color: var(--border);
  }

  .tooltip-right::before {
    left: 100%;
    top: 50%;
    transform: translateY(-50%);
    border-right-color: var(--border);
  }
</style>
