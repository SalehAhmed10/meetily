import { describe, expect, test } from 'bun:test';
import { formatMeetingDate, groupActionItems, type ProjectActionItem } from '../../src/lib/action-items';

const item = (meeting: string, task: string, done = false): ProjectActionItem => ({
  meeting_id: meeting, meeting_title: `${meeting} call`, meeting_date: '2026-11-03T06:17:16Z',
  key: task.toLowerCase(), done, owner: null, task, due: null, timestamp: null,
});

describe('groupActionItems', () => {
  const items = [item('m2', 'Send scope'), item('m2', 'Book demo', true), item('m1', 'Share assets')];

  test('groups by meeting in arrival order and hides done items', () => {
    const groups = groupActionItems(items, false);
    expect(groups.map(g => [g.meetingId, g.items.map(i => i.task)])).toEqual([
      ['m2', ['Send scope']],
      ['m1', ['Share assets']],
    ]);
  });

  test('shows done items when asked and drops meetings with nothing to show', () => {
    expect(groupActionItems(items, true)[0].items.map(i => i.task)).toEqual(['Send scope', 'Book demo']);
    expect(groupActionItems([item('m1', 'Done', true)], false)).toEqual([]);
  });
});

test('formatMeetingDate falls back to the raw text', () => {
  expect(formatMeetingDate('not a date')).toBe('not a date');
  expect(formatMeetingDate('2026-11-03T06:17:16Z')).toContain('2026');
});
