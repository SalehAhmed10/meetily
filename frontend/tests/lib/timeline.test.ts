import { expect, test } from 'bun:test';
import { dateInputValue, groupByStage, knownStages, type TimelineEntry } from '../../src/lib/timeline';

const entry = (id: string, stage: string | null): TimelineEntry => ({
  id, title: id, kind: 'meeting', stage, date: '2026-11-03', has_summary: false,
});

test('groups by stage in the order each stage first happened, ignoring case', () => {
  const groups = groupByStage([
    entry('intro', null),
    entry('interview', 'Round 1'),
    entry('congrats email', 'round 1'),
    entry('tech test', 'Round 2'),
    entry('note', null),
  ]);
  expect(groups.map(g => [g.stage, g.entries.map(e => e.id)])).toEqual([
    [null, ['intro', 'note']],
    ['Round 1', ['interview', 'congrats email']],
    ['Round 2', ['tech test']],
  ]);
});

test('knownStages lists stages once, without the unstaged group', () => {
  expect(knownStages([entry('a', 'Phase 1'), entry('b', null), entry('c', 'phase 1'), entry('d', 'Phase 2')]))
    .toEqual(['Phase 1', 'Phase 2']);
});

test('dateInputValue uses the local date', () => {
  expect(dateInputValue(new Date(2026, 10, 5, 23, 30))).toBe('2026-11-05');
});
