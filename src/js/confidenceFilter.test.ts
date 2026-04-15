/**
 * Confidence Filtering System Tests
 *
 * Comprehensive test suite for confidence-based filtering functionality.
 * Run with: npm test or bun test
 */

import { describe, it, expect, beforeEach } from 'bun:test';
import {
  ConfidenceFilter,
  formatFilterStats,
  getFilterLevelLabel,
  getFilterLevelDescription,
  createConfidenceFilter,
  type ConfidenceFilterConfig,
  type FilterDecision,
} from './confidenceFilter';

describe('ConfidenceFilter', () => {
  let filter: ConfidenceFilter;

  beforeEach(() => {
    filter = new ConfidenceFilter();
  });

  describe('Basic Filtering', () => {
    it('should not filter when level is "none"', () => {
      filter.updateConfig({ level: 'none' });

      const highDecision = filter.shouldFilterSegment('high');
      const mediumDecision = filter.shouldFilterSegment('medium');
      const lowDecision = filter.shouldFilterSegment('low');

      expect(highDecision.shouldFilter).toBe(false);
      expect(mediumDecision.shouldFilter).toBe(false);
      expect(lowDecision.shouldFilter).toBe(false);
    });

    it('should filter only low confidence when level is "low"', () => {
      filter.updateConfig({ level: 'low' });

      const highDecision = filter.shouldFilterSegment('high');
      const mediumDecision = filter.shouldFilterSegment('medium');
      const lowDecision = filter.shouldFilterSegment('low');

      expect(highDecision.shouldFilter).toBe(false);
      expect(mediumDecision.shouldFilter).toBe(false);
      expect(lowDecision.shouldFilter).toBe(true);
      expect(lowDecision.reason).toBe('Low confidence');
    });

    it('should filter low and medium confidence when level is "medium"', () => {
      filter.updateConfig({ level: 'medium' });

      const highDecision = filter.shouldFilterSegment('high');
      const mediumDecision = filter.shouldFilterSegment('medium');
      const lowDecision = filter.shouldFilterSegment('low');

      expect(highDecision.shouldFilter).toBe(false);
      expect(mediumDecision.shouldFilter).toBe(true);
      expect(mediumDecision.reason).toBe('Medium confidence');
      expect(lowDecision.shouldFilter).toBe(true);
      expect(lowDecision.reason).toBe('Low confidence');
    });

    it('should handle undefined confidence as medium', () => {
      filter.updateConfig({ level: 'low' });

      const decision = filter.shouldFilterSegment(undefined);
      expect(decision.shouldFilter).toBe(false);
    });
  });

  describe('Statistics Tracking', () => {
    it('should track total segments correctly', () => {
      filter.shouldFilterSegment('high');
      filter.shouldFilterSegment('medium');
      filter.shouldFilterSegment('low');

      const stats = filter.getStats();
      expect(stats.totalSegments).toBe(3);
    });

    it('should track filtered segments correctly', () => {
      filter.updateConfig({ level: 'low' });

      filter.shouldFilterSegment('high');
      filter.shouldFilterSegment('medium');
      filter.shouldFilterSegment('low');

      const stats = filter.getStats();
      expect(stats.filteredSegments).toBe(1);
      expect(stats.passedSegments).toBe(2);
    });

    it('should calculate filter rate correctly', () => {
      filter.updateConfig({ level: 'low' });

      filter.shouldFilterSegment('high');
      filter.shouldFilterSegment('medium');
      filter.shouldFilterSegment('low');

      const stats = filter.getStats();
      expect(stats.filterRate).toBeCloseTo(0.333, 2);
    });

    it('should track by-level statistics', () => {
      filter.updateConfig({ level: 'medium' });

      filter.shouldFilterSegment('high');
      filter.shouldFilterSegment('medium');
      filter.shouldFilterSegment('medium');
      filter.shouldFilterSegment('low');

      const stats = filter.getStats();
      expect(stats.byLevel.high).toBe(0);
      expect(stats.byLevel.medium).toBe(2);
      expect(stats.byLevel.low).toBe(1);
    });
  });

  describe('Adaptive Filtering', () => {
    it('should disable filtering when filter rate exceeds threshold', () => {
      filter.updateConfig({
        level: 'medium',
        adaptiveEnabled: true,
        adaptiveThreshold: 0.5,
        adaptiveMinSegments: 3,
      });

      // First 2 segments - no adaptive yet
      expect(filter.isAdaptiveDisabled()).toBe(false);
      filter.shouldFilterSegment('medium');
      filter.shouldFilterSegment('low');

      expect(filter.isAdaptiveDisabled()).toBe(false);

      // Third segment - adaptive kicks in, 66% filtered > 50% threshold
      filter.shouldFilterSegment('medium');

      expect(filter.isAdaptiveDisabled()).toBe(true);

      // Should not filter anymore
      const decision = filter.shouldFilterSegment('low');
      expect(decision.shouldFilter).toBe(false);
      expect(decision.adaptiveApplied).toBe(true);
    });

    it('should not disable filtering when below threshold', () => {
      filter.updateConfig({
        level: 'low',
        adaptiveEnabled: true,
        adaptiveThreshold: 0.5,
        adaptiveMinSegments: 3,
      });

      filter.shouldFilterSegment('high');
      filter.shouldFilterSegment('medium');
      filter.shouldFilterSegment('low');

      expect(filter.isAdaptiveDisabled()).toBe(false);
    });

    it('should re-enable when level changes', () => {
      filter.updateConfig({
        level: 'medium',
        adaptiveEnabled: true,
        adaptiveThreshold: 0.5,
        adaptiveMinSegments: 3,
      });

      // Trigger adaptive disable
      filter.shouldFilterSegment('medium');
      filter.shouldFilterSegment('low');
      filter.shouldFilterSegment('medium');

      expect(filter.isAdaptiveDisabled()).toBe(true);

      // Change level
      filter.updateConfig({ level: 'low' });

      expect(filter.isAdaptiveDisabled()).toBe(false);
    });
  });

  describe('Configuration Management', () => {
    it('should update configuration correctly', () => {
      filter.updateConfig({ level: 'medium' });

      const config = filter.getConfig();
      expect(config.level).toBe('medium');
    });

    it('should maintain other config values when updating one', () => {
      filter.updateConfig({
        adaptiveEnabled: false,
        adaptiveThreshold: 0.7,
      });

      filter.updateConfig({ level: 'medium' });

      const config = filter.getConfig();
      expect(config.level).toBe('medium');
      expect(config.adaptiveEnabled).toBe(false);
      expect(config.adaptiveThreshold).toBe(0.7);
    });

    it('should reset to defaults when requested', () => {
      filter.updateConfig({ level: 'medium' });
      filter.updateConfig({ adaptiveEnabled: false });

      // Reset by creating new filter
      const newFilter = new ConfidenceFilter();
      const config = newFilter.getConfig();

      expect(config.level).toBe('low');
      expect(config.adaptiveEnabled).toBe(true);
    });
  });

  describe('Statistics Reset', () => {
    it('should reset statistics correctly', () => {
      filter.shouldFilterSegment('high');
      filter.shouldFilterSegment('medium');
      filter.shouldFilterSegment('low');

      expect(filter.getStats().totalSegments).toBe(3);

      filter.resetStats();

      const stats = filter.getStats();
      expect(stats.totalSegments).toBe(0);
      expect(stats.filteredSegments).toBe(0);
      expect(stats.passedSegments).toBe(0);
      expect(stats.filterRate).toBe(0);
    });

    it('should reset adaptive state when stats are reset', () => {
      filter.updateConfig({
        level: 'medium',
        adaptiveEnabled: true,
        adaptiveThreshold: 0.5,
        adaptiveMinSegments: 3,
      });

      // Trigger adaptive disable
      filter.shouldFilterSegment('medium');
      filter.shouldFilterSegment('low');
      filter.shouldFilterSegment('medium');

      expect(filter.isAdaptiveDisabled()).toBe(true);

      filter.resetStats();

      expect(filter.isAdaptiveDisabled()).toBe(false);
    });
  });

  describe('Filter History', () => {
    it('should store filtered segments in history', () => {
      filter.updateConfig({ level: 'low' });

      const segment = {
        id: 1,
        original: 'test',
        confidence: 'low',
      };

      const decision = filter.filterSegment(segment);
      expect(decision.shouldFilter).toBe(true);

      const history = filter.getFilteredHistory();
      expect(history.length).toBe(1);
      expect(history[0].segment).toEqual(segment);
    });

    it('should not store passed segments in history', () => {
      filter.updateConfig({ level: 'low' });

      const segment = {
        id: 1,
        original: 'test',
        confidence: 'high',
      };

      const decision = filter.filterSegment(segment);
      expect(decision.shouldFilter).toBe(false);

      const history = filter.getFilteredHistory();
      expect(history.length).toBe(0);
    });

    it('should restore segments from history', () => {
      filter.updateConfig({ level: 'low' });

      const segment = {
        id: 1,
        original: 'test',
        confidence: 'low',
      };

      filter.filterSegment(segment);
      const history = filter.getFilteredHistory();
      const timestamp = history[0].filteredAt;

      const restored = filter.restoreFromHistory(timestamp);
      expect(restored).toEqual(segment);

      expect(filter.getFilteredHistory().length).toBe(0);
    });

    it('should clear history', () => {
      filter.updateConfig({ level: 'medium' });

      filter.filterSegment({ id: 1, original: 'test1', confidence: 'low' });
      filter.filterSegment({ id: 2, original: 'test2', confidence: 'medium' });

      expect(filter.getFilteredHistory().length).toBe(2);

      filter.clearHistory();

      expect(filter.getFilteredHistory().length).toBe(0);
    });
  });

  describe('Utility Functions', () => {
    it('should format filter statistics correctly', () => {
      const stats = {
        totalSegments: 10,
        filteredSegments: 3,
        passedSegments: 7,
        filterRate: 0.3,
        byLevel: { high: 0, medium: 2, low: 1 },
        lastUpdated: Date.now(),
      };

      const formatted = formatFilterStats(stats);
      expect(formatted).toBe('3/10 (30.0%)');
    });

    it('should get correct filter level labels', () => {
      expect(getFilterLevelLabel('none')).toBe('None');
      expect(getFilterLevelLabel('low')).toBe('Low');
      expect(getFilterLevelLabel('medium')).toBe('Medium');
    });

    it('should get correct filter level descriptions', () => {
      expect(getFilterLevelDescription('none')).toContain('all translations');
      expect(getFilterLevelDescription('low')).toContain('low-confidence');
      expect(getFilterLevelDescription('medium')).toContain('strict');
    });
  });

  describe('Edge Cases', () => {
    it('should handle rapid config changes', () => {
      filter.updateConfig({ level: 'none' });
      filter.updateConfig({ level: 'low' });
      filter.updateConfig({ level: 'medium' });
      filter.updateConfig({ level: 'none' });

      const decision = filter.shouldFilterSegment('low');
      expect(decision.shouldFilter).toBe(false);
    });

    it('should handle many segments without performance issues', () => {
      filter.updateConfig({ level: 'low' });

      const start = Date.now();
      for (let i = 0; i < 1000; i++) {
        filter.shouldFilterSegment(i % 3 === 0 ? 'low' : 'high');
      }
      const duration = Date.now() - start;

      expect(duration).toBeLessThan(100); // Should be very fast
      expect(filter.getStats().totalSegments).toBe(1000);
    });

    it('should handle history limit correctly', () => {
      filter.updateConfig({ level: 'medium' });

      // Add more than 1000 filtered segments
      for (let i = 0; i < 1500; i++) {
        filter.filterSegment({
          id: i,
          original: `test${i}`,
          confidence: 'low',
        });
      }

      const history = filter.getFilteredHistory();
      expect(history.length).toBe(1000); // Should limit to 1000
    });
  });

  describe('Factory Function', () => {
    it('should create filter with default config', () => {
      const newFilter = createConfidenceFilter();
      const config = newFilter.getConfig();

      expect(config.level).toBe('low');
      expect(config.adaptiveEnabled).toBe(true);
    });

    it('should create filter with custom config', () => {
      const newFilter = createConfidenceFilter({
        level: 'medium',
        adaptiveEnabled: false,
      });
      const config = newFilter.getConfig();

      expect(config.level).toBe('medium');
      expect(config.adaptiveEnabled).toBe(false);
    });
  });
});

describe('ConfidenceFilter Integration Tests', () => {
  describe('Real-World Scenarios', () => {
    it('should handle typical lecture scenario', () => {
      const filter = new ConfidenceFilter({ level: 'low' });

      // Typical lecture: mostly high confidence, some medium
      const confidences = ['high', 'high', 'high', 'medium', 'high', 'high', 'medium', 'high'];

      confidences.forEach(conf => filter.shouldFilterSegment(conf));

      const stats = filter.getStats();
      expect(stats.totalSegments).toBe(8);
      expect(stats.filteredSegments).toBe(0); // No low confidence
      expect(stats.filterRate).toBe(0);
    });

    it('should handle noisy environment scenario', () => {
      const filter = new ConfidenceFilter({
        level: 'low',
        adaptiveEnabled: true,
        adaptiveThreshold: 0.4,
        adaptiveMinSegments: 5,
      });

      // Noisy environment: lots of low confidence
      const confidences = ['low', 'low', 'medium', 'low', 'medium', 'low'];

      confidences.forEach(conf => filter.shouldFilterSegment(conf));

      // Should adaptively disable due to high filter rate
      expect(filter.isAdaptiveDisabled()).toBe(true);
    });

    it('should handle bilingual conversation', () => {
      const filter = new ConfidenceFilter({ level: 'low' });

      // Bilingual: mix of high and medium confidence
      const confidences = ['high', 'high', 'medium', 'high', 'medium', 'high', 'high'];

      confidences.forEach(conf => filter.shouldFilterSegment(conf));

      const stats = filter.getStats();
      expect(stats.filterRate).toBeLessThan(0.3); // Should be reasonable
    });
  });
});
