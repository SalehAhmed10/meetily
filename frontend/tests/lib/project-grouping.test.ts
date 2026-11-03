import { describe, expect, test } from 'bun:test';
import { groupMeetingsByProject, projectFolderId } from '../../src/components/Sidebar/SidebarProvider';

const projects = [
  { id: 'p-acme', name: 'Acme', created_at: '' },
  { id: 'p-empty', name: 'Empty', created_at: '' },
  { id: 'p-zaroba', name: 'Zaroba', created_at: '' },
];

describe('sidebar project grouping', () => {
  test('no projects keeps the flat meeting list', () => {
    const items = groupMeetingsByProject([{ id: 'm1', title: 'Call' }, { id: 'm2', title: 'Sync' }], []);
    expect(items).toEqual([
      { id: 'm1', title: 'Call', type: 'file' },
      { id: 'm2', title: 'Sync', type: 'file' },
    ]);
  });

  test('folders for projects with meetings, in project order, then unassigned meetings', () => {
    const items = groupMeetingsByProject(
      [
        { id: 'm1', title: 'Kickoff', project_id: 'p-zaroba' },
        { id: 'm2', title: 'Notes', project_id: null },
        { id: 'm3', title: 'Scope', project_id: 'p-acme' },
        { id: 'm4', title: 'Review', project_id: 'p-zaroba' },
        { id: 'm5', title: 'Orphan', project_id: 'p-deleted' },
      ],
      projects,
    );
    expect(items.map(i => [i.id, i.title, i.count])).toEqual([
      [projectFolderId('p-acme'), 'Acme', 1],
      [projectFolderId('p-zaroba'), 'Zaroba', 2],
      ['m2', 'Notes', undefined],
      ['m5', 'Orphan', undefined],
    ]);
    expect(items[1].children?.map(c => c.id)).toEqual(['m1', 'm4']);
  });
});
