<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  interface TranscriptMeta {
    filename: string;
    date: string;
    segment_count: number;
    preview: string;
  }

  let {
    onBack,
  }: {
    onBack: () => void;
  } = $props();

  let transcripts: TranscriptMeta[] = $state([]);
  let selectedContent: string | null = $state(null);
  let selectedFilename: string | null = $state(null);
  let loading = $state(true);
  let deleting = $state<string | null>(null);

  onMount(async () => {
    await refreshList();
  });

  async function refreshList() {
    loading = true;
    try {
      transcripts = await invoke<TranscriptMeta[]>('list_transcripts');
    } catch (err) {
      console.error('Failed to load transcripts:', err);
    }
    loading = false;
  }

  async function handleOpen(filename: string) {
    try {
      selectedContent = await invoke<string>('load_transcript', { filename });
      selectedFilename = filename;
    } catch (err) {
      console.error('Failed to load transcript:', err);
    }
  }

  async function handleDelete(filename: string) {
    deleting = filename;
    try {
      await invoke('delete_transcript', { filename });
      transcripts = transcripts.filter((t) => t.filename !== filename);
      if (selectedFilename === filename) {
        selectedContent = null;
        selectedFilename = null;
      }
    } catch (err) {
      console.error('Failed to delete transcript:', err);
    }
    deleting = null;
  }

  function handleBack() {
    if (selectedContent !== null) {
      selectedContent = null;
      selectedFilename = null;
    } else {
      onBack();
    }
  }

  function formatDate(dateStr: string): string {
    // "2026-04-12 14:30:00" -> "Apr 12, 2026 at 2:30 PM"
    const parts = dateStr.split(' ');
    if (parts.length !== 2) return dateStr;
    const [datePart, timePart] = parts;
    const iso = `${datePart}T${timePart}`;
    const d = new Date(iso);
    if (isNaN(d.getTime())) return dateStr;
    return d.toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
    }) + ' at ' + d.toLocaleTimeString('en-US', {
      hour: 'numeric',
      minute: '2-digit',
      hour12: true,
    });
  }
</script>

<div class="saved-view">
  <!-- Header -->
  <div class="saved-header" data-tauri-drag-region>
    <button class="btn-icon" onclick={handleBack} title="Back">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <line x1="19" y1="12" x2="5" y2="12"/>
        <polyline points="12 19 5 12 12 5"/>
      </svg>
    </button>
    <span class="saved-title" data-tauri-drag-region>
      {selectedContent !== null ? selectedFilename?.replace('.txt', '') : 'Saved Transcripts'}
    </span>
  </div>

  <!-- Content -->
  <div class="saved-content">
    {#if selectedContent !== null}
      <!-- Detail view -->
      <div class="detail-view">
        <pre class="detail-text">{selectedContent}</pre>
      </div>
    {:else if loading}
      <div class="empty-state">
        <span>Loading...</span>
      </div>
    {:else if transcripts.length === 0}
      <div class="empty-state">
        <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="var(--text-dim)" stroke-width="1.5">
          <circle cx="12" cy="12" r="10"/>
          <polyline points="12 6 12 12 16 14"/>
        </svg>
        <span>No saved transcripts yet</span>
      </div>
    {:else}
      <!-- List view -->
      <div class="transcript-list">
        {#each transcripts as t (t.filename)}
          <div class="transcript-card" onclick={() => handleOpen(t.filename)}>
            <div class="card-header">
              <span class="card-date">{formatDate(t.date)}</span>
              <div class="card-actions">
                <button
                  class="card-btn"
                  onclick={(e) => { e.stopPropagation(); handleDelete(t.filename); }}
                  disabled={deleting === t.filename}
                  title="Delete"
                >
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <polyline points="3 6 5 6 21 6"/>
                    <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>
                  </svg>
                </button>
              </div>
            </div>
            <div class="card-preview">{t.preview}</div>
            <div class="card-meta">{t.segment_count} segment{t.segment_count !== 1 ? 's' : ''}</div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .saved-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .saved-header {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    height: 44px;
    padding: 0 var(--space-sm);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    user-select: none;
    -webkit-user-select: none;
  }

  .saved-title {
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .saved-content {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-md);
  }

  /* Empty state */
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-md);
    height: 100%;
    color: var(--text-dim);
    font-size: var(--font-size-sm);
  }

  /* Transcript list */
  .transcript-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .transcript-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: var(--space-md);
    cursor: pointer;
    transition: background var(--transition-fast), border-color var(--transition-fast);
  }

  .transcript-card:hover {
    background: var(--bg-hover);
    border-color: var(--border-hover);
  }

  .card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-xs);
  }

  .card-date {
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--text-primary);
  }

  .card-actions {
    display: flex;
    gap: var(--space-xs);
  }

  .card-btn {
    width: 24px;
    height: 24px;
    border-radius: var(--radius-sm);
    border: none;
    background: transparent;
    color: var(--text-dim);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: color var(--transition-fast), background var(--transition-fast);
    padding: 0;
  }

  .card-btn:hover {
    color: var(--danger);
    background: var(--danger-dim);
  }

  .card-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .card-preview {
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    margin-bottom: var(--space-xs);
  }

  .card-meta {
    font-size: var(--font-size-xs);
    color: var(--text-dim);
  }

  /* Detail view */
  .detail-view {
    height: 100%;
  }

  .detail-text {
    font-family: var(--font-family);
    font-size: var(--font-size-sm);
    color: var(--text-primary);
    line-height: 1.8;
    white-space: pre-wrap;
    word-wrap: break-word;
    background: none;
    border: none;
    margin: 0;
    padding: 0;
  }
</style>
