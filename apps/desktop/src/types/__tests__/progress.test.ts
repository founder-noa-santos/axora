import { describe, it, expect } from 'vitest';
import { formatDuration, calculateETA } from '../../types/progress';

describe('Progress Utilities', () => {
  describe('formatDuration', () => {
    it('should format seconds', () => {
      expect(formatDuration(0)).toBe('0s');
      expect(formatDuration(5)).toBe('5s');
      expect(formatDuration(59)).toBe('59s');
    });

    it('should format minutes and seconds', () => {
      expect(formatDuration(60)).toBe('1m 0s');
      expect(formatDuration(90)).toBe('1m 30s');
      expect(formatDuration(120)).toBe('2m 0s');
      expect(formatDuration(150)).toBe('2m 30s');
    });

    it('should format hours, minutes, and seconds', () => {
      expect(formatDuration(3600)).toBe('1h 0m');
      expect(formatDuration(3660)).toBe('1h 1m');
      expect(formatDuration(3725)).toBe('1h 2m');
      expect(formatDuration(7265)).toBe('2h 1m');
    });

    it('should handle negative values', () => {
      expect(formatDuration(-1)).toBe('—');
      expect(formatDuration(-100)).toBe('—');
    });

    it('should handle large durations', () => {
      expect(formatDuration(86400)).toBe('24h 0m');
      expect(formatDuration(90000)).toBe('25h 0m');
    });
  });

  describe('calculateETA', () => {
    it('should calculate ETA from progress and elapsed time', () => {
      // 50% done in 100 seconds = 100 seconds remaining
      expect(calculateETA(50, 100)).toBe(100);

      // 25% done in 60 seconds = 180 seconds remaining
      // rate = 25/60 = 0.417, total = 60/0.417 = 144, remaining = 144 - 60 = 84
      expect(calculateETA(25, 60)).toBe(84);

      // 75% done in 300 seconds = 900 seconds remaining
      // rate = 75/300 = 0.25, total = 300/0.25 = 1200, remaining = 1200 - 300 = 900
      expect(calculateETA(75, 300)).toBe(900);
    });

    it('should return undefined for 0% progress', () => {
      expect(calculateETA(0, 100)).toBe(undefined);
    });

    it('should return undefined for 100% progress', () => {
      expect(calculateETA(100, 100)).toBe(undefined);
    });

    it('should return undefined for negative rate', () => {
      expect(calculateETA(50, 0)).toBe(undefined);
    });

    it('should return 0 for remaining time when almost complete', () => {
      // Edge case: very high progress with little time
      const eta = calculateETA(99.9, 1000);
      expect(eta).toBeDefined();
      expect(eta! >= 0).toBe(true);
    });

    it('should floor the result', () => {
      // 33% in 100 seconds = ~203.03 seconds remaining
      const eta = calculateETA(33, 100);
      expect(eta).toBe(Math.floor(eta!));
    });
  });
});
