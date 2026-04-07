<script lang="ts">
  let { status = 'idle', text = '', showDot = true }: {
    status?: 'ready' | 'error' | 'processing' | 'idle';
    text?: string;
    showDot?: boolean;
  } = $props();
</script>

<div class="status-indicator {status}">
  {#if showDot}
    <span class="status-dot" class:active={status === 'processing'}></span>
  {/if}
  <span>{text || status}</span>
</div>
