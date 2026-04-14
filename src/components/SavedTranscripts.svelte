<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';

  interface TranscriptMeta {
    filename: string;
    date: string;
    segment_count: number;
    preview: string;
  }

  interface SummaryPreview {
    exists: boolean;
    model_used: string | null;
    tier: string | null;
    key_points_count: number | null;
    action_items_count: number | null;
  }

  interface SummaryData {
    version: number;
    transcript_file: string;
    generated_at: string;
    model_used: string;
    tier: string;
    summary: {
      key_points: string[];
      full_summary: string;
      action_items?: { task: string; assignee: string | null; due: string | null }[];
      decisions?: string[];
    };
  }

  let {
    onBack,
    subscriptionTier = 'free',
  }: {
    onBack: () => void;
    subscriptionTier?: 'free' | 'pro';
  } = $props();

  let transcriptsWithSummaries: (TranscriptMeta & { summaryPreview?: SummaryPreview })[] = $state([]);
  let selectedContent: string | null = $state(null);
  let selectedFilename: string | null = $state(null);
  let selectedSummary: SummaryData | null = $state(null);
  let showingSummary = $state(false);
  let loading = $state(true);
  let deleting = $state<string | null>(null);
  let generatingSummary = $state<string | null>(null);
  let summaryStatus = $state('');
  let renaming = $state<string | null>(null);
  let renameValue = $state('');
  let searchQuery = $state('');

  // Filter transcripts based on search query
  let filteredTranscripts = $derived(() => {
    if (!searchQuery.trim()) {
      return transcriptsWithSummaries;
    }
    const query = searchQuery.toLowerCase();
    return transcriptsWithSummaries.filter(t =>
      t.filename.toLowerCase().includes(query) ||
      t.preview.toLowerCase().includes(query) ||
      formatDate(t.date).toLowerCase().includes(query)
    );
  });

  const summaryUnlisteners: UnlistenFn[] = [];

  onMount(async () => {
    await refreshList();

    // Listen for summary events
    const progressUnlisten = await listen<any>('summary-progress', (event) => {
      const payload = event.payload;
      if (typeof payload === 'string') {
        try {
          const data = JSON.parse(payload);
          summaryStatus = data.message || 'Generating...';
        } catch {
          summaryStatus = payload;
        }
      } else if (payload?.message) {
        summaryStatus = payload.message;
      }
    });

    const resultUnlisten = await listen<any>('summary-result', () => {
      // Summary saved — refresh list and show summary
      if (generatingSummary) {
        const filename = generatingSummary;
        generatingSummary = null;
        summaryStatus = '';
        refreshList().then(() => handleViewSummary(filename));
      }
    });

    const errorUnlisten = await listen<any>('summary-error', (event) => {
      console.error('Summary generation failed:', event.payload);
      const payload = event.payload;
      if (typeof payload === 'string') {
        try {
          const data = JSON.parse(payload);
          summaryStatus = data.message || 'Summary generation failed';
        } catch {
          summaryStatus = payload;
        }
      } else {
        summaryStatus = payload?.message || 'Summary generation failed';
      }
      generatingSummary = null;
    });

    summaryUnlisteners.push(progressUnlisten, resultUnlisten, errorUnlisten);
  });

  onDestroy(() => {
    for (const unlisten of summaryUnlisteners) {
      unlisten();
    }
  });

  async function refreshList() {
    loading = true;
    try {
      const transcripts = await invoke<TranscriptMeta[]>('list_transcripts');
      // Check summary status for each transcript in parallel
      const enriched = await Promise.all(
        transcripts.map(async (t) => {
          try {
            const preview = await invoke<SummaryPreview>('check_summary', { filename: t.filename });
            return { ...t, summaryPreview: preview };
          } catch {
            return { ...t, summaryPreview: undefined };
          }
        })
      );
      transcriptsWithSummaries = enriched;
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
      await invoke('delete_summary', { filename }).catch(() => {});
      transcriptsWithSummaries = transcriptsWithSummaries.filter((t) => t.filename !== filename);
      if (selectedFilename === filename) {
        selectedContent = null;
        selectedFilename = null;
      }
    } catch (err) {
      console.error('Failed to delete transcript:', err);
    }
    deleting = null;
  }

  function startRename(filename: string) {
    renaming = filename;
    renameValue = filename.replace('.txt', '');
  }

  async function confirmRename() {
    if (!renaming || !renameValue.trim()) return;
    try {
      const newName = renameValue.trim();
      const newFilename = await invoke<string>('rename_transcript', {
        filename: renaming,
        newName,
      });
      if (selectedFilename === renaming) {
        selectedFilename = newFilename;
      }
      renaming = null;
      renameValue = '';
      await refreshList();
    } catch (err) {
      console.error('Failed to rename:', err);
    }
  }

  function cancelRename() {
    renaming = null;
    renameValue = '';
  }

  function extractLangs(text: string): string {
    // Extract from "[HH:MM:SS] text (en → vi) text" pattern
    const match = text.match(/\((\w+)\s*→\s*(\w+)\)/);
    if (match) return `${match[1]} → ${match[2]}`;
    return '';
  }

  async function handleGenerateSummary(filename: string) {
    generatingSummary = filename;
    summaryStatus = 'Starting summary generation...';
    try {
      // Rust returns immediately; events handle progress/completion
      // Use the actual subscription tier from props
      await invoke('generate_summary', { filename, tier: subscriptionTier });
    } catch (err) {
      console.error('Failed to start summary generation:', err);
      summaryStatus = `Failed: ${typeof err === 'string' ? err : err instanceof Error ? err.message : 'Unknown error'}`;
      generatingSummary = null;
    }
  }

  async function handleViewSummary(filename: string) {
    try {
      // Load transcript content for language detection if not already loaded
      if (!selectedContent || selectedFilename !== filename) {
        selectedContent = await invoke<string>('load_transcript', { filename });
        selectedFilename = filename;
      }
      const result = await invoke<SummaryData | null>('load_summary', { filename });
      if (result) {
        selectedSummary = result;
        showingSummary = true;
      }
    } catch (err) {
      console.error('Failed to load summary:', err);
    }
  }

  function handleBackFromSummary() {
    selectedSummary = null;
    showingSummary = false;
  }

  function handleBack() {
    if (showingSummary) {
      handleBackFromSummary();
    } else if (selectedContent !== null) {
      selectedContent = null;
      selectedFilename = null;
    } else {
      onBack();
    }
  }

  async function handleExportTxt() {
    if (!selectedContent || !selectedFilename) return;
    try {
      // Use the existing content as-is for TXT export
      const blob = new Blob([selectedContent], { type: 'text/plain' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = selectedFilename.replace('.txt', '') + '_export.txt';
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    } catch (err) {
      console.error('Failed to export TXT:', err);
    }
  }

  async function handleExportJson() {
    if (!selectedContent || !selectedFilename) return;
    try {
      // Parse the transcript content into structured JSON
      const lines = selectedContent.split('\n').filter(line => line.trim());
      const segments: { timestamp?: string; original?: string; translated?: string; }[] = [];

      for (const line of lines) {
        const timestampMatch = line.match(/\[(\d{2}:\d{2}:\d{2})\]/);
        const langMatch = line.match(/\((\w+)\s*→\s*(\w+)\)/);

        if (timestampMatch || langMatch) {
          const segment: { timestamp?: string; original?: string; translated?: string; } = {};
          if (timestampMatch) segment.timestamp = timestampMatch[1];
          if (langMatch) {
            // Split by the language pattern and extract original/translated
            const parts = line.split(/\(\w+\s*→\s*\w+\)/);
            if (parts.length >= 2) {
              segment.original = parts[0].replace(/\[\d{2}:\d{2}:\d{2}\]\s*/, '').trim();
              segment.translated = parts[1].trim();
            }
          }
          segments.push(segment);
        }
      }

      const jsonData = {
        filename: selectedFilename,
        exportedAt: new Date().toISOString(),
        segments
      };

      const blob = new Blob([JSON.stringify(jsonData, null, 2)], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = selectedFilename.replace('.txt', '') + '_export.json';
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    } catch (err) {
      console.error('Failed to export JSON:', err);
    }
  }

  async function handleExportCsv() {
    if (!selectedContent || !selectedFilename) return;
    try {
      const lines = selectedContent.split('\n').filter(line => line.trim());
      const csvRows = ['Timestamp,Original,Translated'];

      for (const line of lines) {
        const timestampMatch = line.match(/\[(\d{2}:\d{2}:\d{2})\]/);
        const langMatch = line.match(/\((\w+)\s*→\s*(\w+)\)/);

        if (timestampMatch || langMatch) {
          const timestamp = timestampMatch ? timestampMatch[1] : '';
          const parts = line.split(/\(\w+\s*→\s*\w+\)/);
          let original = '';
          let translated = '';

          if (parts.length >= 2) {
            original = parts[0].replace(/\[\d{2}:\d{2}:\d{2}\]\s*/, '').trim().replace(/"/g, '""');
            translated = parts[1].trim().replace(/"/g, '""');
          }

          // Escape CSV values
          const escapeCsv = (val: string) => `"${val.replace(/"/g, '""')}"`;
          csvRows.push(`${escapeCsv(timestamp)},${escapeCsv(original)},${escapeCsv(translated)}`);
        }
      }

      const csvContent = csvRows.join('\n');
      const blob = new Blob([csvContent], { type: 'text/csv' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = selectedFilename.replace('.txt', '') + '_export.csv';
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    } catch (err) {
      console.error('Failed to export CSV:', err);
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

  function copySummaryJson() {
    if (selectedSummary) {
      navigator.clipboard.writeText(JSON.stringify(selectedSummary, null, 2)).catch((err) => {
        console.error('Failed to copy summary:', err);
      });
    }
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
    {#if (showingSummary && selectedSummary && renaming && selectedFilename) || (selectedContent !== null && renaming === selectedFilename)}
      <div class="header-rename-row">
        <input class="header-rename-input" type="text" bind:value={renameValue} onkeydown={(e) => { if (e.key === 'Enter') confirmRename(); if (e.key === 'Escape') cancelRename(); }} />
        <button class="card-btn rename-confirm-btn" onclick={confirmRename} title="Save">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--success)" stroke-width="2.5"><polyline points="20 6 9 17 4 12"/></svg>
        </button>
        <button class="card-btn rename-cancel-btn" onclick={cancelRename} title="Cancel">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--danger)" stroke-width="2.5"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
        </button>
      </div>
    {:else if showingSummary && selectedSummary}
      <div class="header-detail-row" data-tauri-drag-region>
        <span class="saved-title">
          <span class="saved-title-text">{selectedFilename?.replace('.txt', '')} — Summary</span>
          {#if selectedContent && extractLangs(selectedContent.split('\n')[0] || '')}
            <span class="lang-badge">{extractLangs(selectedContent.split('\n')[0])}</span>
          {/if}
        </span>
        <div class="card-actions">
          <button class="card-btn export-btn" onclick={handleExportTxt} title="Export as TXT" disabled={!selectedContent}>
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
          </button>
          <button class="card-btn export-btn" onclick={handleExportJson} title="Export as JSON" disabled={!selectedContent}>
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/></svg>
          </button>
          <button class="card-btn export-btn" onclick={handleExportCsv} title="Export as CSV" disabled={!selectedContent}>
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="2" y="3" width="20" height="14" rx="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg>
          </button>
          <div class="card-actions-divider"></div>
          <button class="card-btn" onclick={() => selectedFilename && startRename(selectedFilename)} title="Rename">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M17 3a2.828 2.828 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z"/></svg>
          </button>
          <button class="card-btn" onclick={() => selectedFilename && handleDelete(selectedFilename)} disabled={deleting === selectedFilename} title="Delete">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="3 6 5 6 21 6"/>
              <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>
            </svg>
          </button>
        </div>
      </div>
    {:else if selectedContent !== null}
      <div class="header-detail-row" data-tauri-drag-region>
        <span class="saved-title">
          <span class="saved-title-text">{selectedFilename?.replace('.txt', '')}</span>
          {#if selectedContent && extractLangs(selectedContent.split('\n')[0] || '')}
            <span class="lang-badge">{extractLangs(selectedContent.split('\n')[0])}</span>
          {/if}
        </span>
        <div class="card-actions">
          <button class="card-btn export-btn" onclick={handleExportTxt} title="Export as TXT" disabled={!selectedContent}>
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
          </button>
          <button class="card-btn export-btn" onclick={handleExportJson} title="Export as JSON" disabled={!selectedContent}>
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/></svg>
          </button>
          <button class="card-btn export-btn" onclick={handleExportCsv} title="Export as CSV" disabled={!selectedContent}>
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="2" y="3" width="20" height="14" rx="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg>
          </button>
          <div class="card-actions-divider"></div>
          <button class="card-btn" onclick={() => selectedFilename && startRename(selectedFilename)} title="Rename">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M17 3a2.828 2.828 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z"/></svg>
          </button>
          <button class="card-btn" onclick={() => selectedFilename && handleDelete(selectedFilename)} disabled={deleting === selectedFilename} title="Delete">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="3 6 5 6 21 6"/>
              <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>
            </svg>
          </button>
        </div>
      </div>
    {:else}
      <span class="saved-title" data-tauri-drag-region>Saved Transcripts</span>
    {/if}
  </div>

  <!-- Content -->
  <div class="saved-content">
    {#if showingSummary && selectedSummary}
      <!-- Summary view -->
      <div class="summary-view">
        <div class="summary-header">
          <div class="summary-header-left">
            <span class="summary-model">{selectedSummary.model_used}</span>
            <span class="pro-badge">{selectedSummary.tier?.toUpperCase() ?? 'FREE'}</span>
            {#if selectedContent && extractLangs(selectedContent.split('\n')[0] || '')}
              <span class="lang-badge">{extractLangs(selectedContent.split('\n')[0])}</span>
            {/if}
          </div>
          <span class="summary-date">{formatDate(selectedSummary.generated_at)}</span>
        </div>

        <!-- Key Points -->
        {#if selectedSummary.summary.key_points?.length}
          <div class="summary-section">
            <div class="summary-section-title">Key Points</div>
            <ul class="summary-list">
              {#each selectedSummary.summary.key_points as point}
                <li>{point}</li>
              {/each}
            </ul>
          </div>
        {/if}

        <!-- Full Summary -->
        {#if selectedSummary.summary.full_summary}
          <div class="summary-section">
            <div class="summary-section-title">Summary</div>
            <p class="summary-text">{selectedSummary.summary.full_summary}</p>
          </div>
        {/if}

        <!-- Action Items -->
        {#if selectedSummary.summary.action_items?.length}
          <div class="summary-section">
            <div class="summary-section-title">Action Items</div>
            <div class="action-items">
              {#each selectedSummary.summary.action_items as item}
                <div class="action-item">
                  <span class="action-task">{item.task}</span>
                  <span class="action-meta">
                    {#if item.assignee}
                      <span class="action-assignee">{item.assignee}</span>
                    {/if}
                    {#if item.due}
                      <span class="action-due">{item.due}</span>
                    {/if}
                  </span>
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Decisions -->
        {#if selectedSummary.summary.decisions?.length}
          <div class="summary-section">
            <div class="summary-section-title">Decisions</div>
            <ul class="summary-list">
              {#each selectedSummary.summary.decisions as decision}
                <li>{decision}</li>
              {/each}
            </ul>
          </div>
        {/if}

        <!-- Footer actions -->
        <div class="summary-actions">
          {#if generatingSummary === selectedFilename}
            <div class="summary-generating">
              <div class="summary-spinner"></div>
              <span class="summary-status">{summaryStatus || 'Regenerating summary...'}</span>
            </div>
          {:else}
            <button
              class="summary-action-btn"
              onclick={() => {
                if (selectedFilename) {
                  handleGenerateSummary(selectedFilename);
                }
              }}
            >
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/></svg>
              Regenerate
            </button>
          {/if}
          <button class="summary-action-btn" onclick={copySummaryJson}>
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
            Copy JSON
          </button>
        </div>
      </div>

    {:else if selectedContent !== null}
      <!-- Detail view -->
      <div class="detail-view">
        <pre class="detail-text">{selectedContent}</pre>
        <div class="detail-actions">
          {#if generatingSummary === selectedFilename}
            <div class="summary-generating">
              <div class="summary-spinner"></div>
              <span class="summary-status">{summaryStatus || 'Generating summary...'}</span>
            </div>
          {:else if transcriptsWithSummaries.find(t => t.filename === selectedFilename)?.summaryPreview?.exists}
            <button class="summary-btn" onclick={() => selectedFilename && handleViewSummary(selectedFilename)}>
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/></svg>
              View Summary
            </button>
          {:else}
            <button
              class="summary-btn"
              onclick={() => selectedFilename && handleGenerateSummary(selectedFilename)}
            >
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/></svg>
              Generate Summary
            </button>
          {/if}
        </div>
      </div>
    {:else if loading}
      <div class="empty-state">
        <span>Loading...</span>
      </div>
    {:else if transcriptsWithSummaries.length === 0}
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
        <!-- Search bar -->
        <div class="search-bar">
          <svg class="search-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="11" cy="11" r="8"/>
            <path d="M21 21l-4.35-4.35"/>
          </svg>
          <input
            class="search-input"
            type="text"
            placeholder="Search transcripts..."
            bind:value={searchQuery}
          />
          {#if searchQuery}
            <button class="search-clear" onclick={() => searchQuery = ''} title="Clear search">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="18" y1="6" x2="6" y2="18"/>
                <line x1="6" y1="6" x2="18" y2="18"/>
              </svg>
            </button>
          {/if}
        </div>
        {#if filteredTranscripts().length === 0}
          <div class="empty-state">
            <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="var(--text-dim)" stroke-width="1.5">
              <circle cx="12" cy="12" r="10"/>
              <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/>
              <line x1="12" y1="17" x2="12.01" y2="17"/>
            </svg>
            <span>No transcripts match "{searchQuery}"</span>
          </div>
        {/if}
        {#each filteredTranscripts() as t (t.filename)}
          <div class="transcript-card" onclick={() => renaming !== t.filename && handleOpen(t.filename)}>
            <div class="card-header">
              {#if renaming === t.filename}
                <div class="rename-row" onclick={(e) => e.stopPropagation()}>
                  <input
                    class="rename-input"
                    type="text"
                    bind:value={renameValue}
                    onkeydown={(e) => { if (e.key === 'Enter') confirmRename(); if (e.key === 'Escape') cancelRename(); }}
                  />
                  <button class="card-btn" onclick={(e) => { e.stopPropagation(); confirmRename(); }} title="Save">
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="var(--success)" stroke-width="2.5"><polyline points="20 6 9 17 4 12"/></svg>
                  </button>
                  <button class="card-btn" onclick={(e) => { e.stopPropagation(); cancelRename(); }} title="Cancel">
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="var(--danger)" stroke-width="2.5"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
                  </button>
                </div>
              {:else}
                <span class="card-date">{formatDate(t.date)}</span>
                <div class="card-actions">
                  <button
                    class="card-btn"
                    onclick={(e) => { e.stopPropagation(); startRename(t.filename); }}
                    title="Rename"
                  >
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M17 3a2.828 2.828 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z"/></svg>
                  </button>
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
              {/if}
            </div>
            <div class="card-preview">{t.preview}</div>
            <div class="card-meta">
              <span>{t.segment_count} segment{t.segment_count !== 1 ? 's' : ''}</span>
              {#if extractLangs(t.preview)}
                <span class="lang-badge">{extractLangs(t.preview)}</span>
              {/if}
            </div>

            <!-- Summary status row -->
            <div class="card-summary-row">
              {#if generatingSummary === t.filename}
                <div class="summary-generating">
                  <div class="summary-spinner"></div>
                  <span class="summary-status">{summaryStatus || 'Generating summary...'}</span>
                </div>
              {:else if t.summaryPreview?.exists}
                <button
                  class="summary-badge"
                  onclick={(e) => { e.stopPropagation(); handleViewSummary(t.filename); }}
                  title="View summary"
                >
                  <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/></svg>
                  Summary: {t.summaryPreview.key_points_count ?? 0} point{t.summaryPreview.key_points_count !== 1 ? 's' : ''}
                </button>
              {:else}
                <button
                  class="summary-btn"
                  onclick={(e) => { e.stopPropagation(); handleGenerateSummary(t.filename); }}
                >
                  <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/></svg>
                  Generate Summary
                </button>
              {/if}
            </div>
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
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--text-primary);
    min-width: 0;
    flex: 1;
  }

  .saved-title-text {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
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

  .search-bar {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-sm) var(--space-md);
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    margin-bottom: var(--space-sm);
  }

  .search-icon {
    flex-shrink: 0;
    color: var(--text-dim);
  }

  .search-input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: var(--text-primary);
    font-size: var(--font-size-sm);
    font-family: var(--font-family);
  }

  .search-input::placeholder {
    color: var(--text-dim);
  }

  .search-clear {
    flex-shrink: 0;
    width: 20px;
    height: 20px;
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

  .search-clear:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
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
    color: var(--text-primary);
    background: var(--bg-hover);
  }

  .card-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .export-btn {
    color: var(--accent);
  }

  .export-btn:hover {
    background: rgba(99, 140, 255, 0.1);
    color: var(--accent-hover);
  }

  .card-actions-divider {
    width: 1px;
    height: 16px;
    background: var(--border);
    margin: 0 4px;
  }

  .header-rename-btn {
    flex-shrink: 0;
  }

  .header-detail-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex: 1;
    min-width: 0;
  }

  .rename-row {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: 1;
  }

  .header-rename-row {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 1;
    min-width: 0;
  }

  .header-rename-input {
    flex: 1;
    min-width: 0;
    height: 30px;
    font-size: var(--font-size-sm);
    padding: 0 10px;
    border: 1px solid var(--accent);
    border-radius: var(--radius-md);
    background: var(--bg-primary);
    color: var(--text-primary);
    outline: none;
    transition: border-color var(--transition-fast), box-shadow var(--transition-fast);
  }

  .header-rename-input:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-dim, rgba(99, 102, 241, 0.15));
  }

  .rename-confirm-btn,
  .rename-cancel-btn {
    width: 30px;
    height: 30px;
    border-radius: var(--radius-sm);
    flex-shrink: 0;
  }

  .rename-input {
    flex: 1;
    min-width: 0;
    font-size: var(--font-size-xs);
    padding: 2px 6px;
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    background: var(--bg-primary);
    color: var(--text-primary);
    outline: none;
  }

  .rename-input:focus {
    border-color: var(--accent);
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

  /* Summary row in list cards */
  .card-summary-row {
    margin-top: var(--space-xs);
  }

  .summary-badge {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: var(--font-size-xs);
    font-weight: 500;
    color: var(--accent);
    background: var(--accent-dim, rgba(99, 102, 241, 0.1));
    border: 1px solid var(--accent-border, rgba(99, 102, 241, 0.2));
    border-radius: var(--radius-md);
    padding: 5px 12px;
    cursor: pointer;
    transition: background var(--transition-fast), border-color var(--transition-fast);
  }

  .summary-badge:hover {
    background: var(--accent-dim-hover, rgba(99, 102, 241, 0.18));
    border-color: var(--accent-border-hover, rgba(99, 102, 241, 0.35));
  }

  .summary-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: var(--font-size-xs);
    font-weight: 500;
    color: var(--text-secondary);
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: 5px 12px;
    cursor: pointer;
    transition: background var(--transition-fast), border-color var(--transition-fast), color var(--transition-fast);
  }

  .summary-btn:hover {
    color: var(--text-primary);
    background: var(--bg-hover);
    border-color: var(--border-hover);
  }

  .summary-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .summary-status {
    font-size: var(--font-size-xs);
    color: var(--text-dim);
    font-style: italic;
  }

  .summary-generating {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 0;
  }

  .summary-spinner {
    width: 14px;
    height: 14px;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: summary-spin 0.8s linear infinite;
    flex-shrink: 0;
  }

  @keyframes summary-spin {
    to { transform: rotate(360deg); }
  }

  .lang-badge {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-size: 10px;
    font-weight: 600;
    color: var(--text-dim);
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 1px 8px;
    letter-spacing: 0.5px;
    text-transform: uppercase;
  }

  /* Detail view */
  .detail-view {
    height: 100%;
    display: flex;
    flex-direction: column;
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
    flex: 1;
    overflow-y: auto;
  }

  .detail-actions {
    display: flex;
    gap: var(--space-sm);
    padding-top: var(--space-md);
    border-top: 1px solid var(--border);
    margin-top: var(--space-md);
    flex-shrink: 0;
  }

  /* Summary view */
  .summary-view {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }

  .summary-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-sm);
    flex-wrap: wrap;
  }

  .summary-header-left {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }

  .summary-model {
    font-size: var(--font-size-xs);
    font-weight: 600;
    color: var(--text-secondary);
  }

  .pro-badge {
    font-size: 0.6rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--accent);
    background: var(--accent-dim, rgba(99, 102, 241, 0.1));
    padding: 1px 6px;
    border-radius: 4px;
    line-height: 1.5;
  }

  .summary-date {
    font-size: var(--font-size-xs);
    color: var(--text-dim);
    margin-left: auto;
  }

  .summary-section {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: var(--space-md);
  }

  .summary-section-title {
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: var(--space-sm);
  }

  .summary-list {
    margin: 0;
    padding-left: var(--space-md);
    list-style: disc;
  }

  .summary-list li {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    line-height: 1.6;
    margin-bottom: var(--space-xs);
  }

  .summary-text {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    line-height: 1.7;
    margin: 0;
  }

  .action-items {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .action-item {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-sm);
    padding: var(--space-xs) 0;
  }

  .action-task {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
  }

  .action-meta {
    display: flex;
    gap: var(--space-sm);
    flex-shrink: 0;
  }

  .action-assignee {
    font-size: var(--font-size-xs);
    color: var(--accent);
    font-weight: 500;
  }

  .action-due {
    font-size: var(--font-size-xs);
    color: var(--text-dim);
  }

  .summary-actions {
    display: flex;
    gap: var(--space-sm);
    padding-top: var(--space-sm);
    border-top: 1px solid var(--border);
    margin-top: var(--space-sm);
  }

  .summary-action-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: var(--font-size-xs);
    font-weight: 500;
    color: var(--text-secondary);
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: 5px 12px;
    cursor: pointer;
    transition: background var(--transition-fast), border-color var(--transition-fast), color var(--transition-fast);
  }

  .summary-action-btn:hover {
    color: var(--text-primary);
    background: var(--bg-hover);
    border-color: var(--border-hover);
  }
</style>
