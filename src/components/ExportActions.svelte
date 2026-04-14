<!--
  Export Actions Component

  Provides export functionality for transcripts with different formats.
  Used by SavedTranscripts component.

  Props:
    - filename: string - The transcript filename to export
    - content: string - The transcript content to export
    - buttonStyle: 'inline' | 'dropdown' - How to display the export buttons
-->

<script lang="ts">
  let {
    filename,
    content,
    buttonStyle = 'inline',
    onExport
  }: {
    filename: string;
    content: string;
    buttonStyle?: 'inline' | 'dropdown';
    onExport?: (format: string) => void;
  } = $props();

  let showExportMenu = $state(false);
  let exporting = $state(false);

  async function handleExportTxt() {
    if (exporting) return;
    exporting = true;

    try {
      const blob = new Blob([content], { type: 'text/plain' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `${filename}.txt`;
      a.click();
      URL.revokeObjectURL(url);

      onExport?.('txt');
    } finally {
      exporting = false;
    }
  }

  async function handleExportJson() {
    if (exporting) return;
    exporting = true;

    try {
      // Try to parse as JSON for better formatting
      let jsonContent;
      try {
        const parsed = JSON.parse(content);
        jsonContent = JSON.stringify(parsed, null, 2);
      } catch {
        // If not valid JSON, wrap in a simple structure
        jsonContent = JSON.stringify({
          filename,
          content,
          exported_at: new Date().toISOString()
        }, null, 2);
      }

      const blob = new Blob([jsonContent], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `${filename}.json`;
      a.click();
      URL.revokeObjectURL(url);

      onExport?.('json');
    } finally {
      exporting = false;
    }
  }

  async function handleExportCsv() {
    if (exporting) return;
    exporting = true;

    try {
      // Parse transcript content and create CSV
      const lines = content.split('\n').filter(line => line.trim());
      const segments: { original: string; translated: string }[] = [];

      let currentOriginal = '';
      let currentTranslated = '';

      for (const line of lines) {
        const originalMatch = line.match(/^Original:\s*(.*)$/);
        const translatedMatch = line.match(/^Translated:\s*(.*)$/);

        if (originalMatch) {
          if (currentOriginal || currentTranslated) {
            segments.push({ original: currentOriginal, translated: currentTranslated });
          }
          currentOriginal = originalMatch[1];
          currentTranslated = '';
        } else if (translatedMatch) {
          currentTranslated = translatedMatch[1];
        } else if (line.trim()) {
          // Additional content for current segment
          if (currentTranslated) {
            currentTranslated += '\n' + line;
          } else {
            currentOriginal += '\n' + line;
          }
        }
      }

      // Add last segment
      if (currentOriginal || currentTranslated) {
        segments.push({ original: currentOriginal, translated: currentTranslated });
      }

      // Create CSV
      const headers = ['Original', 'Translated'];
      const csvRows = [
        headers.join(','),
        ...segments.map(s =>
          `"${s.original.replace(/"/g, '""')}", "${s.translated.replace(/"/g, '""')}"`
        )
      ];

      const csvContent = csvRows.join('\n');
      const blob = new Blob([csvContent], { type: 'text/csv' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `${filename}.csv`;
      a.click();
      URL.revokeObjectURL(url);

      onExport?.('csv');
    } finally {
      exporting = false;
    }
  }

  function toggleExportMenu() {
    showExportMenu = !showExportMenu;
  }
</script>

{#if buttonStyle === 'dropdown'}
  <div class="export-dropdown">
    <button class="export-button" onclick={toggleExportMenu} class:open={showExportMenu}>
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <path d="M21 15v4a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2v-4"/>
        <polyline points="7 10 12 15 17 10"/>
        <line x1="12" y1="15" x2="12" y2="3"/>
      </svg>
      <span class="export-label">Export</span>
      <svg class="export-arrow" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" class:rotated={showExportMenu}>
        <polyline points="6 9 12 15 9 6"/>
      </svg>
    </button>

    {#if showExportMenu}
      <div class="export-menu">
        <button class="export-option" onclick={handleExportTxt} disabled={exporting}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V4a2 2 0 0 0-2-2z"/>
            <polyline points="14 6 14 14 10 14"/>
            <line x1="16" y1="8" x2="8" y2="8"/>
            <line x1="16" y1="16" x2="8" y2="16"/>
            <line x1="10" y1="16" x2="10" y2="8"/>
          </svg>
          <span class="export-option-text">
            {exporting ? 'Exporting...' : 'Export as TXT'}
          </span>
        </button>

        <button class="export-option" onclick={handleExportJson} disabled={exporting}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"/>
            <polyline points="14 6 14 14 10 14"/>
            <path d="M6 10l4 4 4-4"/>
            <path d="M14 6v4a2 2 0 0 0-2-2"/>
          </svg>
          <span class="export-option-text">
            {exporting ? 'Exporting...' : 'Export as JSON'}
          </span>
        </button>

        <button class="export-option" onclick={handleExportCsv} disabled={exporting}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8"/>
            <path d="M2 12h20"/>
            <path d="M14 6v4a2 2 0 0 0-2-2"/>
            <path d="M10 12v4"/>
            <path d="M6 12l4 4 4-4"/>
          </svg>
          <span class="export-option-text">
            {exporting ? 'Exporting...' : 'Export as CSV'}
          </span>
        </button>
      </div>
    {/if}
  </div>
{:else}
  <div class="export-actions">
    <button class="export-btn" onclick={handleExportTxt} disabled={exporting} title="Export as TXT">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V4a2 2 0 0 0-2-2z"/>
        <polyline points="14 6 14 14 10 14"/>
        <line x1="16" y1="8" x2="8" y2="8"/>
        <line x1="16" y1="16" x2="8" y2="16"/>
        <line x1="10" y1="16" x2="10" y2="8"/>
      </svg>
    </button>

    <button class="export-btn" onclick={handleExportJson} disabled={exporting} title="Export as JSON">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"/>
        <polyline points="14 6 14 14 10 14"/>
        <path d="M6 10l4 4 4-4"/>
        <path d="M14 6v4a2 2 0 0 0-2-2"/>
      </svg>
    </button>

    <button class="export-btn" onclick={handleExportCsv} disabled={exporting} title="Export as CSV">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8"/>
        <path d="M2 12h20"/>
        <path d="M14 6v4a2 2 0 0 0-2-2"/>
        <path d="M10 12v4"/>
        <path d="M6 12l4 4 4-4"/>
      </svg>
    </button>
  </div>
{/if}

<style>
  .export-dropdown {
    position: relative;
    display: inline-block;
  }

  .export-button {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .export-button:hover {
    border-color: var(--border-hover);
    color: var(--text-primary);
  }

  .export-button.open {
    border-color: var(--border-accent);
    background: var(--bg-hover);
  }

  .export-button svg:first-child {
    width: 16px;
    height: 16px;
  }

  .export-label {
    font-size: var(--font-size-sm);
  }

  .export-arrow {
    width: 14px;
    height: 14px;
    transition: transform 0.2s ease;
  }

  .export-arrow.rotated {
    transform: rotate(180deg);
  }

  .export-menu {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    background: var(--bg-solid);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    z-index: var(--z-dropdown);
    min-width: 180px;
    padding: 4px;
  }

  .export-option {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
    cursor: pointer;
    transition: all 0.2s ease;
    text-align: left;
  }

  .export-option:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .export-option:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .export-option svg {
    width: 16px;
    height: 16px;
    flex-shrink: 0;
  }

  .export-option-text {
    flex: 1;
  }

  /* Inline export actions */
  .export-actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .export-btn {
    width: 32px;
    height: 32px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
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

  .export-btn:hover:not(:disabled) {
    color: var(--text-secondary);
    background: var(--bg-hover);
    border-color: var(--border-hover);
  }

  .export-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .export-btn svg {
    width: 14px;
    height: 14px;
  }
</style>
