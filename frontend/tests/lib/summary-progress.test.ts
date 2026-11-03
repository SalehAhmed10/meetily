import { describe, expect, test } from 'bun:test';
import { estimateSecondsLeft, formatDuration } from '../../src/components/MeetingDetails/SummaryProgressView';

describe('summary progress estimates', () => {
  test('formats durations as seconds, then minutes and seconds', () => {
    expect(formatDuration(25_400)).toBe('25s');
    expect(formatDuration(791_600)).toBe('13m 12s');
    expect(formatDuration(-5)).toBe('0s');
  });

  test('time left uses speed between samples and counts down from the latest one', () => {
    const first = { at: 0, chars: 0 };
    const last = { at: 5_000, chars: 500 }; // 100 chars/s
    // 2,500 chars left at 100 chars/s = 25s, measured at the latest sample
    expect(estimateSecondsLeft(first, last, 3_000, 5_000)).toBe(25);
    // The 1s clock keeps ticking between 5s polls: the estimate must fall, not rise
    expect(estimateSecondsLeft(first, last, 3_000, 8_000)).toBe(22);
  });

  test('no estimate without two distinct samples', () => {
    const sample = { at: 1_000, chars: 100 };
    expect(estimateSecondsLeft(sample, sample, 3_000, 2_000)).toBeNull();
  });
});
