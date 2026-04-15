/**
 * Confidence Filtering System for Auralis Translation Pipeline
 *
 * Provides production-ready confidence-based filtering to prevent low-quality
 * translations from reaching the UI and saved transcripts.
 *
 * Features:
 * - Configurable filter levels with persistence
 * - Adaptive filtering to prevent over-filtering
 * - Comprehensive statistics and monitoring
 * - Filter preview mode for testing
 * - Show/restore filtered content functionality
 * - Performance-optimized filtering logic
 */

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type ConfidenceFilterLevel = 'none' | 'low' | 'medium';

export interface ConfidenceFilterConfig {
  /** Current filter level */
  level: ConfidenceFilterLevel;
  /** Enable adaptive filtering (auto-disable if too aggressive) */
  adaptiveEnabled: boolean;
  /** Maximum percentage of segments that can be filtered before adaptive disables (0-1) */
  adaptiveThreshold: number;
  /** Minimum segments before adaptive kicks in */
  adaptiveMinSegments: number;
  /** Show filtered segments in UI (ghosted/dimmed) */
  showFiltered: boolean;
}

export interface ConfidenceFilterStats {
  /** Total segments processed */
  totalSegments: number;
  /** Segments filtered out */
  filteredSegments: number;
  /** Segments that passed filter */
  passedSegments: number;
  /** Current filter rate (0-1) */
  filterRate: number;
  /** Filter rate by confidence level */
  byLevel: Record<string, number>;
  /** Timestamp of last update */
  lastUpdated: number;
}

export interface FilterDecision {
  /** Whether the segment should be filtered out */
  shouldFilter: boolean;
  /** Reason for filtering */
  reason?: string;
  /** Current filter level used */
  filterLevel: ConfidenceFilterLevel;
  /** Whether adaptive filtering was applied */
  adaptiveApplied: boolean;
}

export interface FilteredSegment {
  /** Original segment data */
  segment: any;
  /** Timestamp when filtered */
  filteredAt: number;
  /** Reason for filtering */
  reason: string;
  /** Filter level used */
  filterLevel: ConfidenceFilterLevel;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const DEFAULT_CONFIG: ConfidenceFilterConfig = {
  level: 'low', // Filter out low confidence by default
  adaptiveEnabled: true,
  adaptiveThreshold: 0.5, // Disable if >50% filtered
  adaptiveMinSegments: 10, // Need 10 segments before adaptive
  showFiltered: false,
};

const STORAGE_KEY = 'auralis-confidence-filter-config';

// ---------------------------------------------------------------------------
// Confidence Filter Class
// ---------------------------------------------------------------------------

export class ConfidenceFilter {
  private config: ConfidenceFilterConfig;
  private stats: ConfidenceFilterStats;
  private filteredHistory: FilteredSegment[] = [];
  private adaptiveDisabled: boolean = false;
  private segmentHistory: ('high' | 'medium' | 'low')[] = [];

  constructor(config?: Partial<ConfidenceFilterConfig>) {
    this.config = { ...DEFAULT_CONFIG, ...config };
    this.stats = this.createEmptyStats();
    this.loadConfig();
  }

  // -----------------------------------------------------------------------
  // Public API
  // -----------------------------------------------------------------------

  /**
   * Determine if a segment should be filtered based on confidence
   *
   * @param confidence - Segment confidence level
   * @returns Filter decision with reasoning
   */
  shouldFilterSegment(confidence?: 'high' | 'medium' | 'low'): FilterDecision {
    const decision: FilterDecision = {
      shouldFilter: false,
      filterLevel: this.config.level,
      adaptiveApplied: false,
    };

    // Default confidence if not provided
    const segmentConfidence = confidence ?? 'medium';

    // Track segment history for adaptive filtering
    this.segmentHistory.push(segmentConfidence);
    if (this.segmentHistory.length > 100) {
      this.segmentHistory.shift(); // Keep last 100
    }

    // Check if adaptive has disabled filtering
    if (this.adaptiveDisabled) {
      decision.reason = 'Adaptive filtering disabled (too aggressive)';
      this.updateStats(segmentConfidence, false);
      return decision;
    }

    // Apply filter based on level
    switch (this.config.level) {
      case 'none':
        decision.shouldFilter = false;
        decision.reason = 'Filtering disabled';
        break;

      case 'low':
        decision.shouldFilter = segmentConfidence === 'low';
        decision.reason = segmentConfidence === 'low' ? 'Low confidence' : 'Passed low filter';
        break;

      case 'medium':
        decision.shouldFilter = segmentConfidence === 'low' || segmentConfidence === 'medium';
        decision.reason =
          segmentConfidence === 'low'
            ? 'Low confidence'
            : segmentConfidence === 'medium'
            ? 'Medium confidence'
            : 'Passed medium filter';
        break;
    }

    // Update statistics
    this.updateStats(segmentConfidence, decision.shouldFilter);

    // Check adaptive filtering
    if (this.config.adaptiveEnabled && decision.shouldFilter) {
      decision.adaptiveApplied = this.checkAdaptive();
      if (this.adaptiveDisabled) {
        decision.shouldFilter = false;
        decision.reason = 'Adaptive filtering disabled (too aggressive)';
      }
    }

    return decision;
  }

  /**
   * Filter a segment and store in history if filtered
   *
   * @param segment - Segment to potentially filter
   * @returns Filter decision
   */
  filterSegment(segment: any): FilterDecision {
    const decision = this.shouldFilterSegment(segment.confidence);

    if (decision.shouldFilter) {
      this.filteredHistory.push({
        segment: { ...segment },
        filteredAt: Date.now(),
        reason: decision.reason ?? 'Unknown',
        filterLevel: decision.filterLevel,
      });

      // Limit history size
      if (this.filteredHistory.length > 1000) {
        this.filteredHistory.shift();
      }
    }

    return decision;
  }

  /**
   * Get current filter statistics
   */
  getStats(): ConfidenceFilterStats {
    return { ...this.stats };
  }

  /**
   * Get filter configuration
   */
  getConfig(): ConfidenceFilterConfig {
    return { ...this.config };
  }

  /**
   * Update filter configuration
   */
  updateConfig(updates: Partial<ConfidenceFilterConfig>): void {
    this.config = { ...this.config, ...updates };
    this.saveConfig();

    // Reset adaptive state if level changes
    if (updates.level !== undefined) {
      this.adaptiveDisabled = false;
      this.segmentHistory = [];
    }
  }

  /**
   * Get filtered segment history
   */
  getFilteredHistory(): FilteredSegment[] {
    return [...this.filteredHistory];
  }

  /**
   * Restore a previously filtered segment
   */
  restoreFromHistory(timestamp: number): any | null {
    const index = this.filteredHistory.findIndex((f) => f.filteredAt === timestamp);
    if (index !== -1) {
      const restored = this.filteredHistory[index];
      this.filteredHistory.splice(index, 1);
      return restored.segment;
    }
    return null;
  }

  /**
   * Clear filtered history
   */
  clearHistory(): void {
    this.filteredHistory = [];
  }

  /**
   * Reset statistics
   */
  resetStats(): void {
    this.stats = this.createEmptyStats();
    this.segmentHistory = [];
    this.adaptiveDisabled = false;
  }

  /**
   * Enable/disable adaptive filtering
   */
  setAdaptiveEnabled(enabled: boolean): void {
    this.config.adaptiveEnabled = enabled;
    this.saveConfig();
    if (enabled) {
      this.adaptiveDisabled = false;
    }
  }

  /**
   * Get whether adaptive filtering is currently disabled
   */
  isAdaptiveDisabled(): boolean {
    return this.adaptiveDisabled;
  }

  // -----------------------------------------------------------------------
  // Private Methods
  // -----------------------------------------------------------------------

  /**
   * Create empty statistics object
   */
  private createEmptyStats(): ConfidenceFilterStats {
    return {
      totalSegments: 0,
      filteredSegments: 0,
      passedSegments: 0,
      filterRate: 0,
      byLevel: {
        high: 0,
        medium: 0,
        low: 0,
      },
      lastUpdated: Date.now(),
    };
  }

  /**
   * Update filter statistics
   */
  private updateStats(
    confidence: 'high' | 'medium' | 'low',
    wasFiltered: boolean
  ): void {
    this.stats.totalSegments++;
    this.stats.lastUpdated = Date.now();

    if (wasFiltered) {
      this.stats.filteredSegments++;
      this.stats.byLevel[confidence]++;
    } else {
      this.stats.passedSegments++;
    }

    this.stats.filterRate =
      this.stats.totalSegments > 0
        ? this.stats.filteredSegments / this.stats.totalSegments
        : 0;
  }

  /**
   * Check and apply adaptive filtering
   *
   * Returns true if adaptive filtering was applied (disabled filter)
   */
  private checkAdaptive(): boolean {
    if (!this.config.adaptiveEnabled) {
      return false;
    }

    // Need minimum segments before adaptive kicks in
    if (this.stats.totalSegments < this.config.adaptiveMinSegments) {
      return false;
    }

    // Check if filter rate is too high
    if (this.stats.filterRate >= this.config.adaptiveThreshold) {
      console.warn(
        `[ConfidenceFilter] Filter rate ${(this.stats.filterRate * 100).toFixed(1)}% exceeds threshold ${(this.config.adaptiveThreshold * 100).toFixed(1)}%, disabling filtering`
      );
      this.adaptiveDisabled = true;
      return true;
    }

    return false;
  }

  /**
   * Load configuration from localStorage
   */
  private loadConfig(): void {
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (stored) {
        const parsed = JSON.parse(stored);
        this.config = { ...DEFAULT_CONFIG, ...parsed };
      }
    } catch (err) {
      console.error('[ConfidenceFilter] Failed to load config:', err);
    }
  }

  /**
   * Save configuration to localStorage
   */
  private saveConfig(): void {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(this.config));
    } catch (err) {
      console.error('[ConfidenceFilter] Failed to save config:', err);
    }
  }
}

// ---------------------------------------------------------------------------
// Utility Functions
// ---------------------------------------------------------------------------

/**
 * Create a default confidence filter instance
 */
export function createConfidenceFilter(
  config?: Partial<ConfidenceFilterConfig>
): ConfidenceFilter {
  return new ConfidenceFilter(config);
}

/**
 * Format filter statistics for display
 */
export function formatFilterStats(stats: ConfidenceFilterStats): string {
  const percentage = (stats.filterRate * 100).toFixed(1);
  return `${stats.filteredSegments}/${stats.totalSegments} (${percentage}%)`;
}

/**
 * Get human-readable label for filter level
 */
export function getFilterLevelLabel(level: ConfidenceFilterLevel): string {
  const labels: Record<ConfidenceFilterLevel, string> = {
    none: 'None',
    low: 'Low',
    medium: 'Medium',
  };
  return labels[level];
}

/**
 * Get description for filter level
 */
export function getFilterLevelDescription(level: ConfidenceFilterLevel): string {
  const descriptions: Record<ConfidenceFilterLevel, string> = {
    none: 'Show all translations regardless of confidence',
    low: 'Hide low-confidence translations (may have errors)',
    medium: 'Hide low and medium-confidence translations (strict filtering)',
  };
  return descriptions[level];
}

// ---------------------------------------------------------------------------
// React Hook (for Svelte integration)
// ---------------------------------------------------------------------------

/**
 * Create a reactive confidence filter state
 *
 * Usage in Svelte component:
 * ```svelte
 * <script>
 *   import { createConfidenceFilterState } from './js/confidenceFilter';
 *
 *   const filterState = createConfidenceFilterState();
 * </script>
 *
 * <p>Filter: {getFilterLevelLabel($filterState.config.level)}</p>
 * <p>Stats: {formatFilterStats($filterState.stats)}</p>
 * ```
 */
export function createConfidenceFilterState(
  config?: Partial<ConfidenceFilterConfig>
) {
  let filter = $state(new ConfidenceFilter(config));
  let stats = $derived(filter.getStats());
  let configState = $derived(filter.getConfig());

  return {
    get filter() {
      return filter;
    },
    get stats() {
      return stats;
    },
    get config() {
      return configState;
    },
    updateConfig: (updates: Partial<ConfidenceFilterConfig>) => {
      filter.updateConfig(updates);
    },
    resetStats: () => {
      filter.resetStats();
    },
    clearHistory: () => {
      filter.clearHistory();
    },
  };
}

// ---------------------------------------------------------------------------
// Exports
// ---------------------------------------------------------------------------

export default {
  ConfidenceFilter,
  createConfidenceFilter,
  formatFilterStats,
  getFilterLevelLabel,
  getFilterLevelDescription,
  createConfidenceFilterState,
};
