<script lang="ts">
  import Tooltip from './Tooltip.svelte';

  /**
   * ConfidenceSettings - Streamlined confidence management UI
   *
   * Features:
   * - Clean preset-based configuration
   * - Visual confidence level indicators
   * - Collapsible advanced controls
   * - Glassmorphism design system integration
   * - Accessibility-first design
   */

  export interface ConfidenceSettings {
    filterLevel: 'none' | 'low' | 'medium' | 'high';
    highThreshold: number;
    mediumThreshold: number;
    lowThreshold: number;
    showConfidenceScores: boolean;
    perLanguageOverrides: Record<string, Partial<ConfidenceSettings>>;
  }

  let {
    settings,
    disabled = false,
    onChange,
    onReset
  }: {
    settings: ConfidenceSettings;
    disabled?: boolean;
    onChange?: (settings: ConfidenceSettings) => void;
    onReset?: () => void;
  } = $props();

  let showAdvanced = $state(false);

  const filterPresets = [
    {
      id: 'none',
      name: 'Show All',
      description: 'Display all transcript segments',
      icon: '🔓',
      color: 'var(--text-dim)'
    },
    {
      id: 'low',
      name: 'Basic Filter',
      description: 'Hide very low confidence segments',
      icon: '🟡',
      color: 'var(--warning)'
    },
    {
      id: 'medium',
      name: 'Quality Filter',
      description: 'Only show medium+ confidence',
      icon: '🟠',
      color: 'var(--warning)'
    },
    {
      id: 'high',
      name: 'Strict Filter',
      description: 'Only show high confidence segments',
      icon: '🔒',
      color: 'var(--success)'
    }
  ];

  function setFilterLevel(level: 'none' | 'low' | 'medium' | 'high') {
    settings.filterLevel = level;
    onChange?.(settings);
  }

  function updateThreshold(level: 'high' | 'medium' | 'low', value: number) {
    settings[`${level}Threshold`] = value;
    onChange?.(settings);
  }

  function resetToDefaults() {
    settings.highThreshold = 0.85;
    settings.mediumThreshold = 0.70;
    settings.lowThreshold = 0.50;
    settings.filterLevel = 'none';
    settings.showConfidenceScores = false;
    onReset?.();
    onChange?.(settings);
  }
</script>

<div class="confidence-settings" class:disabled={disabled}>
  <!-- Header with icon -->
  <div class="settings-header">
    <div class="header-icon">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="2">
        <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z"/>
      </svg>
    </div>
    <div class="header-content">
      <h3 class="header-title">Confidence Filter</h3>
      <p class="header-description">Improve translation quality by filtering low-confidence segments</p>
    </div>
  </div>

  <!-- Filter Presets Grid -->
  <div class="presets-grid">
    {#each filterPresets as preset}
      {@const isActive = settings.filterLevel === preset.id}
      <button
        class="preset-card"
        class:active={isActive}
        onclick={() => setFilterLevel(preset.id as any)}
        disabled={disabled}
        aria-label={preset.name}
        aria-pressed={isActive}
      >
        <div class="preset-icon" style:color={preset.color}>
          {preset.icon}
        </div>
        <div class="preset-info">
          <span class="preset-name">{preset.name}</span>
          <span class="preset-desc">{preset.description}</span>
        </div>
        {#if isActive}
          <div class="preset-check">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <polyline points="20 6 9 17 4 12"/>
            </svg>
          </div>
        {/if}
      </button>
    {/each}
  </div>

  <!-- Current Status Display -->
  <div class="status-card">
    <div class="status-main">
      <div class="status-indicator" class:active={settings.filterLevel !== 'none'}>
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z"/>
        </svg>
      </div>
      <div class="status-content">
        <span class="status-label">Current Filter</span>
        <span class="status-value">
          {#if settings.filterLevel === 'none'}
            Showing all segments
          {:else if settings.filterLevel === 'low'}
            Filtering below {Math.round(settings.lowThreshold * 100)}%
          {:else if settings.filterLevel === 'medium'}
            Filtering below {Math.round(settings.mediumThreshold * 100)}%
          {:else}
            Filtering below {Math.round(settings.highThreshold * 100)}%
          {/if}
        </span>
      </div>
    </div>
    <button
      class="action-btn"
      onclick={() => showAdvanced = !showAdvanced}
      aria-label="Toggle advanced settings"
    >
      {showAdvanced ? 'Simple' : 'Advanced'}
    </button>
  </div>

  <!-- Advanced Controls -->
  {#if showAdvanced}
    <div class="advanced-section">
      <div class="advanced-header">
        <span class="advanced-title">Threshold Configuration</span>
        <Tooltip content="Adjust confidence percentages for each filter level">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10"/>
            <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/>
            <line x1="12" y1="17" x2="12.01" y2="17"/>
          </svg>
        </Tooltip>
      </div>

      <div class="threshold-list">
        <!-- High Confidence -->
        <div class="threshold-item">
          <div class="threshold-info">
            <div class="threshold-label">
              <span class="threshold-dot high"></span>
              <span>High Confidence</span>
            </div>
            <span class="threshold-percent">{Math.round(settings.highThreshold * 100)}%</span>
          </div>
          <input
            type="range"
            min="70"
            max="95"
            step="1"
            value={settings.highThreshold * 100}
            oninput={(e) => updateThreshold('high', parseInt(e.target.value) / 100)}
            disabled={disabled}
            class="threshold-slider high"
            style="--fill: {(settings.highThreshold * 100 - 70) / 25 * 100}%"
            aria-label="High confidence threshold: {Math.round(settings.highThreshold * 100)}%"
          />
        </div>

        <!-- Medium Confidence -->
        <div class="threshold-item">
          <div class="threshold-info">
            <div class="threshold-label">
              <span class="threshold-dot medium"></span>
              <span>Medium Confidence</span>
            </div>
            <span class="threshold-percent">{Math.round(settings.mediumThreshold * 100)}%</span>
          </div>
          <input
            type="range"
            min="50"
            max="85"
            step="1"
            value={settings.mediumThreshold * 100}
            oninput={(e) => updateThreshold('medium', parseInt(e.target.value) / 100)}
            disabled={disabled}
            class="threshold-slider medium"
            style="--fill: {(settings.mediumThreshold * 100 - 50) / 35 * 100}%"
            aria-label="Medium confidence threshold: {Math.round(settings.mediumThreshold * 100)}%"
          />
        </div>

        <!-- Low Confidence -->
        <div class="threshold-item">
          <div class="threshold-info">
            <div class="threshold-label">
              <span class="threshold-dot low"></span>
              <span>Low Confidence</span>
            </div>
            <span class="threshold-percent">{Math.round(settings.lowThreshold * 100)}%</span>
          </div>
          <input
            type="range"
            min="30"
            max="70"
            step="1"
            value={settings.lowThreshold * 100}
            oninput={(e) => updateThreshold('low', parseInt(e.target.value) / 100)}
            disabled={disabled}
            class="threshold-slider low"
            style="--fill: {(settings.lowThreshold * 100 - 30) / 40 * 100}%"
            aria-label="Low confidence threshold: {Math.round(settings.lowThreshold * 100)}%"
          />
        </div>
      </div>

      <!-- Show Scores Toggle -->
      <div class="toggle-row">
        <div class="toggle-info">
          <span class="toggle-label">Show confidence scores</span>
          <span class="toggle-desc">Display percentages in transcript</span>
        </div>
        <button
          class="toggle-switch"
          class:active={settings.showConfidenceScores}
          onclick={() => {
            settings.showConfidenceScores = !settings.showConfidenceScores;
            onChange?.(settings);
          }}
          disabled={disabled}
          aria-label="Toggle confidence scores display"
          aria-pressed={settings.showConfidenceScores}
        >
          <span class="toggle-thumb"></span>
        </button>
      </div>
    </div>
  {/if}

  <!-- Reset Button -->
  <div class="settings-footer">
    <button
      class="reset-btn"
      onclick={resetToDefaults}
      disabled={disabled}
    >
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 12"/>
        <path d="M3 3v9h9"/>
      </svg>
      Reset to Defaults
    </button>
  </div>
</div>

<style>
  .confidence-settings {
    display: flex;
    flex-direction: column;
    gap: var(--space-lg);
    padding: var(--space-lg);
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    backdrop-filter: blur(24px) saturate(1.4);
    transition: opacity var(--transition-fast);
  }

  .confidence-settings.disabled {
    opacity: 0.5;
    pointer-events: none;
  }

  /* Header */
  .settings-header {
    display: flex;
    align-items: flex-start;
    gap: var(--space-md);
    padding-bottom: var(--space-md);
    border-bottom: 1px solid var(--border);
  }

  .header-icon {
    width: 36px;
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--accent-dim);
    border: 1px solid var(--border-accent);
    border-radius: var(--radius-md);
    flex-shrink: 0;
  }

  .header-icon svg {
    flex-shrink: 0;
  }

  .header-content {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    flex: 1;
  }

  .header-title {
    font-size: var(--font-size-base);
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .header-description {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    margin: 0;
    line-height: 1.4;
  }

  /* Presets Grid */
  .presets-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: var(--space-sm);
  }

  .preset-card {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-md);
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all var(--transition-fast);
    text-align: left;
    position: relative;
    overflow: hidden;
  }

  .preset-card:hover:not(:disabled) {
    background: var(--bg-hover);
    border-color: var(--border-hover);
    transform: translateY(-1px);
    box-shadow: var(--shadow-sm);
  }

  .preset-card.active {
    background: var(--accent-dim);
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent-dim);
  }

  .preset-card:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .preset-icon {
    font-size: 20px;
    flex-shrink: 0;
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .preset-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    min-width: 0;
  }

  .preset-name {
    font-weight: 600;
    font-size: var(--font-size-sm);
    color: var(--text-primary);
  }

  .preset-desc {
    font-size: var(--font-size-xs);
    color: var(--text-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .preset-check {
    width: 20px;
    height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--accent);
    border-radius: var(--radius-sm);
    color: white;
    flex-shrink: 0;
  }

  /* Status Card */
  .status-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-md);
    padding: var(--space-md);
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }

  .status-main {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    flex: 1;
    min-width: 0;
  }

  .status-indicator {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text-dim);
    flex-shrink: 0;
    transition: all var(--transition-fast);
  }

  .status-indicator.active {
    background: var(--accent-dim);
    border-color: var(--accent);
    color: var(--accent);
  }

  .status-content {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .status-label {
    font-size: var(--font-size-xs);
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .status-value {
    font-size: var(--font-size-sm);
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* Advanced Section */
  .advanced-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
    padding: var(--space-md);
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    animation: fadeIn 0.2s ease;
  }

  .advanced-header {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    padding-bottom: var(--space-sm);
    border-bottom: 1px solid var(--border);
  }

  .advanced-title {
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--text-primary);
  }

  .threshold-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }

  .threshold-item {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .threshold-info {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .threshold-label {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-primary);
  }

  .threshold-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .threshold-dot.high {
    background: var(--success);
  }

  .threshold-dot.medium {
    background: var(--warning);
  }

  .threshold-dot.low {
    background: var(--danger);
  }

  .threshold-percent {
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--accent);
    font-variant-numeric: tabular-nums;
  }

  .threshold-slider {
    width: 100%;
    -webkit-appearance: none;
    appearance: none;
    height: 6px;
    border-radius: 3px;
    outline: none;
    cursor: pointer;
    background: linear-gradient(
      to right,
      var(--success) 0%,
      var(--success) var(--fill, 50%),
      rgba(255, 255, 255, 0.12) var(--fill, 50%),
      rgba(255, 255, 255, 0.12) 100%
    );
    transition: all var(--transition-fast);
    border: none;
  }

  .threshold-slider.medium {
    background: linear-gradient(
      to right,
      var(--warning) 0%,
      var(--warning) var(--fill, 50%),
      rgba(255, 255, 255, 0.12) var(--fill, 50%),
      rgba(255, 255, 255, 0.12) 100%
    );
  }

  .threshold-slider.low {
    background: linear-gradient(
      to right,
      var(--danger) 0%,
      var(--danger) var(--fill, 50%),
      rgba(255, 255, 255, 0.12) var(--fill, 50%),
      rgba(255, 255, 255, 0.12) 100%
    );
  }

  .threshold-slider:hover {
    height: 8px;
  }

  .threshold-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: white;
    cursor: pointer;
    border: none;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
    transition: all var(--transition-fast);
  }

  .threshold-slider::-webkit-slider-thumb:hover {
    transform: scale(1.1);
    box-shadow: 0 2px 12px rgba(99, 140, 255, 0.5);
  }

  .threshold-slider::-moz-range-thumb {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: white;
    cursor: pointer;
    border: none;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
    transition: all var(--transition-fast);
  }

  .threshold-slider::-moz-range-thumb:hover {
    transform: scale(1.1);
    box-shadow: 0 2px 12px rgba(99, 140, 255, 0.5);
  }

  /* Toggle Row */
  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-md);
    padding-top: var(--space-sm);
    border-top: 1px solid var(--border);
  }

  .toggle-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
  }

  .toggle-label {
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-primary);
  }

  .toggle-desc {
    font-size: var(--font-size-xs);
    color: var(--text-dim);
  }

  .toggle-switch {
    width: 44px;
    height: 24px;
    border-radius: 12px;
    border: none;
    background: rgba(255, 255, 255, 0.1);
    position: relative;
    cursor: pointer;
    transition: background var(--transition-fast);
    flex-shrink: 0;
  }

  .toggle-switch.active {
    background: var(--accent);
  }

  .toggle-thumb {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: white;
    transition: transform var(--transition-fast);
    pointer-events: none;
  }

  .toggle-switch.active .toggle-thumb {
    transform: translateX(20px);
  }

  .toggle-switch:disabled {
    opacity: 0.4;
    cursor: default;
  }

  /* Footer */
  .settings-footer {
    display: flex;
    justify-content: center;
    padding-top: var(--space-md);
    border-top: 1px solid var(--border);
  }

  /* Action Button (Advanced/Simple toggle) */
  .action-btn {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    height: 32px;
    padding: 0 var(--space-md);
    background: transparent;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
    font-weight: 500;
    font-family: var(--font-family);
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    white-space: nowrap;
  }

  .action-btn:hover:not(:disabled) {
    background: var(--bg-hover);
    border-color: var(--border-hover);
    color: var(--text-primary);
  }

  .action-btn:active:not(:disabled) {
    transform: scale(0.98);
  }

  .action-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Reset Button */
  .reset-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-xs);
    height: 36px;
    padding: 0 var(--space-md);
    background: transparent;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
    font-weight: 500;
    font-family: var(--font-family);
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    white-space: nowrap;
  }

  .reset-btn svg {
    flex-shrink: 0;
  }

  .reset-btn:hover:not(:disabled) {
    background: rgba(255, 77, 77, 0.1);
    border-color: rgba(255, 77, 77, 0.3);
    color: var(--danger);
  }

  .reset-btn:active:not(:disabled) {
    transform: scale(0.98);
  }

  .reset-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Animations */
  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: translateY(-8px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  /* Accessibility */
  @media (prefers-reduced-motion: reduce) {
    .confidence-settings,
    .preset-card,
    .threshold-slider,
    .toggle-switch,
    .advanced-section,
    .status-indicator {
      transition: none;
      animation: none;
    }
  }

  @media (prefers-contrast: high) {
    .confidence-settings,
    .preset-card,
    .status-card,
    .advanced-section {
      border-width: 2px;
    }
  }

  /* Responsive adjustments */
  @media (max-width: 640px) {
    .presets-grid {
      grid-template-columns: 1fr;
    }

    .settings-header {
      flex-direction: column;
      align-items: flex-start;
    }

    .header-icon {
      width: 32px;
      height: 32px;
    }
  }
</style>
