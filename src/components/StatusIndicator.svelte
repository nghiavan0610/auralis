<script lang="ts">
  export let status: 'ready' | 'error' | 'processing' | 'idle' = 'idle';
  export let text: string = '';
  export let showDot: boolean = true;
</script>

<div class="status-indicator {status}">
  {#if showDot}
    <span class="status-dot" class:active={status === 'processing'}></span>
  {/if}
  <span>{text || status}</span>
</div>
