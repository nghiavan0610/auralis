/**
 * Confidence Score Management for Auralis Speech Recognition
 *
 * Provides production-ready confidence aggregation, smoothing, and
 * threshold management for both cloud (Soniox) and offline (MLX Whisper) modes.
 *
 * Features:
 * - Ensemble confidence aggregation
 * - Exponential moving average smoothing
 * - Adaptive thresholds (per-user, per-language, per-noise-level)
 * - Comprehensive monitoring and metrics
 * - UI integration helpers
 */

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface ConfidenceMetrics {
  /** Transcription confidence (text accuracy) */
  transcription: number;
  /** Language detection confidence */
  languageDetection: number;
  /** Combined overall confidence score */
  overall: number;
  /** Timestamp of confidence calculation */
  timestamp: number;
  /** Number of tokens/samples used for calculation */
  sampleCount: number;
}

export interface ConfidenceThresholds {
  /** High confidence threshold (default: 0.85) */
  high: number;
  /** Medium confidence threshold (default: 0.60) */
  medium: number;
  /** Low confidence threshold (default: 0.40) */
  low: number;
}

export type ConfidenceLevel = 'very-high' | 'high' | 'medium' | 'low' | 'very-low';

export interface NoiseLevel {
  type: 'quiet' | 'moderate' | 'noisy';
  confidence: number;
}

export interface UserConfidenceProfile {
  userId: string;
  baselineConfidence: number;
  totalUtterances: number;
  confidenceHistory: number[];
  lastUpdated: number;
}

export interface LanguagePairAdjustments {
  [key: string]: number; // e.g., "en-vi": -0.05
}

// ---------------------------------------------------------------------------
// Confidence Aggregation
// ---------------------------------------------------------------------------

/**
 * Aggregate confidence scores from multiple tokens using hybrid approach.
 *
 * Combines:
 * - Temporal weighting (recent tokens more important)
 * - Token length weighting (longer tokens typically more confident)
 * - Minimum confidence penalty (weakest link)
 */
export class ConfidenceAggregator {
  private readonly temporalWeight: number;
  private readonly lengthWeight: number;
  private readonly minimumWeight: number;

  constructor(
    temporalWeight: number = 0.7,
    lengthWeight: number = 0.2,
    minimumWeight: number = 0.1
  ) {
    const total = temporalWeight + lengthWeight + minimumWeight;
    if (Math.abs(total - 1.0) > 0.01) {
      throw new Error(`Weights must sum to 1.0, got ${total}`);
    }

    this.temporalWeight = temporalWeight;
    this.lengthWeight = lengthWeight;
    this.minimumWeight = minimumWeight;
  }

  /**
   * Aggregate token-level confidence scores into single value.
   *
   * @param scores Array of confidence scores (0-1 range)
   * @param tokenLengths Optional array of token lengths for length weighting
   * @returns Aggregated confidence or undefined if no valid scores
   */
  aggregate(
    scores: number[],
    tokenLengths?: number[]
  ): number | undefined {
    // Validate input
    if (scores.length === 0) {
      return undefined;
    }

    // Filter invalid scores
    const validScores = this.filterValidScores(scores);
    if (validScores.length === 0) {
      return undefined;
    }

    // Calculate weighted components
    const temporalScore = this.calculateTemporalScore(validScores);
    const lengthScore = this.calculateLengthScore(validScores, tokenLengths);
    const minimumScore = Math.min(...validScores);

    // Combine using weights
    const aggregated =
      temporalScore * this.temporalWeight +
      lengthScore * this.lengthWeight +
      minimumScore * this.minimumWeight;

    // Clamp to valid range
    return Math.max(0.0, Math.min(1.0, aggregated));
  }

  /**
   * Filter out invalid confidence scores.
   */
  private filterValidScores(scores: number[]): number[] {
    return scores.filter(
      score =>
        typeof score === 'number' &&
        !isNaN(score) &&
        isFinite(score) &&
        score >= 0.0 &&
        score <= 1.0
    );
  }

  /**
   * Calculate temporally-weighted average (recent tokens favored).
   */
  private calculateTemporalScore(scores: number[]): number {
    if (scores.length === 0) return 0.5;
    if (scores.length === 1) return scores[0];

    // Weight last 30% of tokens 2x
    const weightRatio = scores.length > 3 ? 0.3 : 0.0;
    const splitIndex = Math.floor(scores.length * (1 - weightRatio));

    let weightedSum = 0.0;
    let totalWeight = 0.0;

    for (let i = 0; i < scores.length; i++) {
      const tokenWeight = i >= splitIndex ? 2.0 : 1.0;
      weightedSum += scores[i] * tokenWeight;
      totalWeight += tokenWeight;
    }

    return totalWeight > 0 ? weightedSum / totalWeight : 0.5;
  }

  /**
   * Calculate length-weighted average (longer tokens favored).
   */
  private calculateLengthScore(
    scores: number[],
    tokenLengths?: number[]
  ): number {
    if (!tokenLengths || tokenLengths.length !== scores.length) {
      // Fallback to simple average if no length data
      return scores.reduce((a, b) => a + b, 0) / scores.length;
    }

    // Weight by token length (normalized)
    const avgLength = tokenLengths.reduce((a, b) => a + b, 0) / tokenLengths.length;

    let weightedSum = 0.0;
    let totalWeight = 0.0;

    for (let i = 0; i < scores.length; i++) {
      // Longer tokens get slight boost (max 2x weight)
      const lengthFactor = Math.min(2.0, tokenLengths[i] / avgLength);
      const weight = Math.max(0.5, lengthFactor);

      weightedSum += scores[i] * weight;
      totalWeight += weight;
    }

    return totalWeight > 0 ? weightedSum / totalWeight : 0.5;
  }
}

// ---------------------------------------------------------------------------
// Confidence Smoothing
// ---------------------------------------------------------------------------

/**
 * Smooth confidence scores using exponential moving average.
 *
 * Reduces jitter and provides more stable confidence estimates
 * across time windows.
 */
export class ConfidenceSmoother {
  private ema: number | null = null;
  private readonly alpha: number; // Smoothing factor (0.2-0.4)

  constructor(alpha: number = 0.3) {
    if (alpha < 0.0 || alpha > 1.0) {
      throw new Error(`Alpha must be between 0 and 1, got ${alpha}`);
    }
    this.alpha = alpha;
  }

  /**
   * Smooth a new confidence value.
   *
   * @param newConfidence New confidence score to smooth
   * @returns Smoothed confidence value
   */
  smooth(newConfidence: number): number {
    if (this.ema === null) {
      this.ema = newConfidence;
    } else {
      this.ema = this.alpha * newConfidence + (1 - this.alpha) * this.ema;
    }
    return this.ema;
  }

  /**
   * Get current smoothed value without updating.
   */
  getCurrent(): number | undefined {
    return this.ema;
  }

  /**
   * Reset smoothing state.
   */
  reset(): void {
    this.ema = null;
  }
}

// ---------------------------------------------------------------------------
// Adaptive Threshold Management
// ---------------------------------------------------------------------------

/**
 * Manage adaptive confidence thresholds based on various factors.
 *
 * Supports:
 * - Per-user baseline adaptation
 * - Per-language pair adjustments
 * - Per-noise level adaptation
 */
export class AdaptiveThresholdManager {
  private userProfiles: Map<string, UserConfidenceProfile> = new Map();
  private languageAdjustments: LanguagePairAdjustments;
  private readonly baseThresholds: ConfidenceThresholds;
  private readonly minHistorySize: number = 10;
  private readonly maxHistorySize: number = 100;

  constructor(
    baseThresholds?: Partial<ConfidenceThresholds>,
    languageAdjustments?: LanguagePairAdjustments
  ) {
    this.baseThresholds = {
      high: 0.85,
      medium: 0.60,
      low: 0.40,
      ...baseThresholds,
    };

    // Default language pair adjustments
    this.languageAdjustments = {
      // Harder pairs (more lenient)
      'en-vi': -0.05,
      'en-ja': -0.08,
      'en-zh': -0.07,
      'en-ko': -0.06,
      'en-th': -0.07,

      // Easier pairs (stricter)
      'en-es': +0.03,
      'en-fr': +0.02,
      'en-de': +0.02,
      'en-pt': +0.02,
      'en-it': +0.02,

      ...languageAdjustments,
    };
  }

  /**
   * Get adaptive threshold for a specific user and context.
   */
  getThreshold(
    userId: string,
    sourceLang: string,
    targetLang: string,
    noiseLevel: NoiseLevel = { type: 'moderate', confidence: 0.7 }
  ): ConfidenceThresholds {
    const baseMedium = this.getUserBaseline(userId);
    const languageAdjusted = this.getLanguageAdjustedThreshold(
      baseMedium,
      sourceLang,
      targetLang
    );
    const noiseAdjusted = this.getNoiseAdjustedThreshold(
      languageAdjusted,
      noiseLevel
    );

    return {
      high: Math.min(0.95, noiseAdjusted + 0.25),
      medium: noiseAdjusted,
      low: Math.max(0.20, noiseAdjusted - 0.20),
    };
  }

  /**
   * Get user-specific baseline confidence.
   */
  private getUserBaseline(userId: string): number {
    const profile = this.userProfiles.get(userId);

    if (!profile || profile.confidenceHistory.length < this.minHistorySize) {
      return this.baseThresholds.medium;
    }

    // Calculate user's 25th percentile confidence
    const sorted = [...profile.confidenceHistory].sort((a, b) => a - b);
    const p25 = sorted[Math.floor(sorted.length * 0.25)];

    // Adaptive threshold: user's p25 + 0.1 buffer
    return Math.max(0.40, Math.min(0.80, p25 + 0.1));
  }

  /**
   * Apply language pair adjustments to threshold.
   */
  private getLanguageAdjustedThreshold(
    baseThreshold: number,
    sourceLang: string,
    targetLang: string
  ): number {
    const key = `${sourceLang}-${targetLang}`;
    const adjustment = this.languageAdjustments[key] || 0;

    return Math.max(0.30, Math.min(0.90, baseThreshold + adjustment));
  }

  /**
   * Apply noise level adjustments to threshold.
   */
  private getNoiseAdjustedThreshold(
    baseThreshold: number,
    noiseLevel: NoiseLevel
  ): number {
    const adjustments: Record<string, number> = {
      quiet: +0.05,      // Stricter in quiet environments
      moderate: 0.0,     // No adjustment
      noisy: -0.10,      // More lenient in noise
    };

    const adjustment = adjustments[noiseLevel.type] || 0.0;

    return Math.max(0.30, Math.min(0.90, baseThreshold + adjustment));
  }

  /**
   * Record a confidence score for a user (for adaptive learning).
   */
  recordConfidence(userId: string, confidence: number): void {
    let profile = this.userProfiles.get(userId);

    if (!profile) {
      profile = {
        userId,
        baselineConfidence: 0.6,
        totalUtterances: 0,
        confidenceHistory: [],
        lastUpdated: Date.now(),
      };
      this.userProfiles.set(userId, profile);
    }

    // Update history
    profile.confidenceHistory.push(confidence);
    profile.totalUtterances++;
    profile.lastUpdated = Date.now();

    // Trim history
    if (profile.confidenceHistory.length > this.maxHistorySize) {
      profile.confidenceHistory.shift();
    }

    // Update baseline periodically
    if (profile.totalUtterances % 20 === 0) {
      this.updateUserBaseline(profile);
    }
  }

  /**
   * Update user's baseline confidence from history.
   */
  private updateUserBaseline(profile: UserConfidenceProfile): void {
    if (profile.confidenceHistory.length < this.minHistorySize) {
      return;
    }

    const sorted = [...profile.confidenceHistory].sort((a, b) => a - b);
    const median = sorted[Math.floor(sorted.length * 0.5)];

    profile.baselineConfidence = median;
  }

  /**
   * Get user profile (for debugging/analytics).
   */
  getUserProfile(userId: string): UserConfidenceProfile | undefined {
    return this.userProfiles.get(userId);
  }
}

// ---------------------------------------------------------------------------
// Confidence Classification
// ---------------------------------------------------------------------------

/**
 * Classify confidence score into discrete levels.
 */
export function classifyConfidence(
  confidence: number,
  thresholds: ConfidenceThresholds
): ConfidenceLevel {
  if (confidence >= thresholds.high) {
    return 'very-high';
  } else if (confidence >= thresholds.medium) {
    return 'high';
  } else if (confidence >= thresholds.low) {
    return 'medium';
  } else if (confidence >= 0.25) {
    return 'low';
  } else {
    return 'very-low';
  }
}

/**
 * Get human-readable label for confidence level.
 */
export function getConfidenceLabel(level: ConfidenceLevel): string {
  const labels: Record<ConfidenceLevel, string> = {
    'very-high': 'Very High',
    'high': 'High',
    'medium': 'Medium',
    'low': 'Low',
    'very-low': 'Very Low',
  };
  return labels[level];
}

/**
 * Get color for confidence level (for UI indicators).
 */
export function getConfidenceColor(level: ConfidenceLevel): string {
  const colors: Record<ConfidenceLevel, string> = {
    'very-high': '#10b981', // green-500
    'high': '#34d399',      // green-400
    'medium': '#fbbf24',    // amber-400
    'low': '#f87171',       // red-400
    'very-low': '#ef4444',  // red-500
  };
  return colors[level];
}

// ---------------------------------------------------------------------------
// Confidence Metrics
// ---------------------------------------------------------------------------

/**
 * Combine transcription and language detection confidences.
 */
export function combineConfidences(
  transcription: number,
  languageDetection: number,
  transcriptionWeight: number = 0.7
  // languageDetectionWeight implicitly = 1 - transcriptionWeight
): number {
  return (
    transcription * transcriptionWeight +
    languageDetection * (1 - transcriptionWeight)
  );
}

/**
 * Create comprehensive confidence metrics object.
 */
export function createConfidenceMetrics(
  transcription: number,
  languageDetection: number,
  sampleCount: number
): ConfidenceMetrics {
  return {
    transcription,
    languageDetection,
    overall: combineConfidences(transcription, languageDetection),
    timestamp: Date.now(),
    sampleCount,
  };
}

// ---------------------------------------------------------------------------
// Confidence Monitoring
// ---------------------------------------------------------------------------

/**
 * Monitor confidence metrics for analytics and debugging.
 */
export class ConfidenceMonitor {
  private metrics = {
    totalUtterances: 0,
    lowConfidenceCount: 0,
    averageConfidence: 0.0,
    confidenceByLanguage: {} as Record<string, number[]>,
    confidenceByLevel: {} as Record<ConfidenceLevel, number>,
  };

  /**
   * Record a confidence measurement.
   */
  recordUtterance(
    confidence: number,
    language: string,
    thresholds: ConfidenceThresholds
  ): void {
    this.metrics.totalUtterances++;

    // Track low confidence
    if (confidence < thresholds.medium) {
      this.metrics.lowConfidenceCount++;
    }

    // Update average
    this.metrics.averageConfidence =
      (this.metrics.averageConfidence * (this.metrics.totalUtterances - 1) +
        confidence) /
      this.metrics.totalUtterances;

    // Track by language
    if (!this.metrics.confidenceByLanguage[language]) {
      this.metrics.confidenceByLanguage[language] = [];
    }
    this.metrics.confidenceByLanguage[language].push(confidence);

    // Track by level
    const level = classifyConfidence(confidence, thresholds);
    this.metrics.confidenceByLevel[level] =
      (this.metrics.confidenceByLevel[level] || 0) + 1;
  }

  /**
   * Get monitoring metrics.
   */
  getMetrics() {
    const byLanguage: Record<
      string,
      { average: number; count: number; min: number; max: number }
    > = {};

    for (const [lang, confs] of Object.entries(
      this.metrics.confidenceByLanguage
    )) {
      if (confs.length > 0) {
        byLanguage[lang] = {
          average: confs.reduce((a, b) => a + b, 0) / confs.length,
          count: confs.length,
          min: Math.min(...confs),
          max: Math.max(...confs),
        };
      }
    }

    return {
      totalUtterances: this.metrics.totalUtterances,
      lowConfidenceCount: this.metrics.lowConfidenceCount,
      lowConfidenceRate:
        this.metrics.totalUtterances > 0
          ? this.metrics.lowConfidenceCount / this.metrics.totalUtterances
          : 0,
      averageConfidence: this.metrics.averageConfidence,
      confidenceByLanguage: byLanguage,
      confidenceByLevel: { ...this.metrics.confidenceByLevel },
    };
  }

  /**
   * Reset monitoring metrics.
   */
  reset(): void {
    this.metrics = {
      totalUtterances: 0,
      lowConfidenceCount: 0,
      averageConfidence: 0.0,
      confidenceByLanguage: {},
      confidenceByLevel: {},
    };
  }
}

// ---------------------------------------------------------------------------
// UI Integration Helpers
// ---------------------------------------------------------------------------

/**
 * Handle low confidence with appropriate user feedback.
 */
export interface LowConfidenceHandler {
  onLowConfidence: (confidence: number, level: ConfidenceLevel) => void;
  showIndicator: (confidence: number, level: ConfidenceLevel) => void;
}

export function handleConfidence(
  confidence: number,
  thresholds: ConfidenceThresholds,
  handler: LowConfidenceHandler
): void {
  const level = classifyConfidence(confidence, thresholds);

  // Always show indicator
  handler.showIndicator(confidence, level);

  // Trigger callback for low/very-low confidence
  if (level === 'low' || level === 'very-low') {
    handler.onLowConfidence(confidence, level);
  }
}

/**
 * Create confidence badge data for UI.
 */
export interface ConfidenceBadgeData {
  score: number;
  level: ConfidenceLevel;
  label: string;
  color: string;
  visible: boolean;
}

export function createConfidenceBadge(
  confidence: number,
  thresholds: ConfidenceThresholds,
  showThreshold: number = 0.60
): ConfidenceBadgeData {
  const level = classifyConfidence(confidence, thresholds);

  return {
    score: confidence,
    level,
    label: getConfidenceLabel(level),
    color: getConfidenceColor(level),
    visible: confidence < showThreshold,
  };
}

// ---------------------------------------------------------------------------
// Error Handling
// ---------------------------------------------------------------------------

/**
 * Validate and handle confidence scores with fallbacks.
 */
export function validateConfidence(
  confidence: number | undefined | null
): number {
  const DEFAULT_CONFIDENCE = 0.6;
  const MIN_CONFIDENCE = 0.1;
  const MAX_CONFIDENCE = 1.0;

  // Handle missing/invalid
  if (
    confidence === undefined ||
    confidence === null ||
    typeof confidence !== 'number' ||
    isNaN(confidence) ||
    !isFinite(confidence)
  ) {
    console.warn('[Confidence] Invalid confidence, using default');
    return DEFAULT_CONFIDENCE;
  }

  // Clamp to valid range
  if (confidence < MIN_CONFIDENCE) {
    console.warn(
      `[Confidence] Extremely low confidence: ${confidence}, clamping to ${MIN_CONFIDENCE}`
    );
    return MIN_CONFIDENCE;
  }

  if (confidence > MAX_CONFIDENCE) {
    console.warn(
      `[Confidence] Confidence > 1.0: ${confidence}, clamping to ${MAX_CONFIDENCE}`
    );
    return MAX_CONFIDENCE;
  }

  return confidence;
}

// ---------------------------------------------------------------------------
// Exports
// ---------------------------------------------------------------------------

export default {
  ConfidenceAggregator,
  ConfidenceSmoother,
  AdaptiveThresholdManager,
  ConfidenceMonitor,
  classifyConfidence,
  getConfidenceLabel,
  getConfidenceColor,
  combineConfidences,
  createConfidenceMetrics,
  handleConfidence,
  createConfidenceBadge,
  validateConfidence,
};
