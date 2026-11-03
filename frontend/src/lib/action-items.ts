/** One action item from a meeting summary, as api_get_project_action_items returns it */
export interface ProjectActionItem {
  meeting_id: string;
  meeting_title: string;
  meeting_date: string;
  key: string;
  done: boolean;
  owner: string | null;
  task: string;
  due: string | null;
  timestamp: string | null;
}

export interface MeetingActionItems {
  meetingId: string;
  title: string;
  date: string;
  items: ProjectActionItem[];
}

/** Items grouped by meeting in the order they arrive (newest meeting first); done items hidden unless asked */
export function groupActionItems(items: ProjectActionItem[], showDone: boolean): MeetingActionItems[] {
  const groups = new Map<string, MeetingActionItems>();
  for (const item of items) {
    if (item.done && !showDone) continue;
    let group = groups.get(item.meeting_id);
    if (!group) {
      group = { meetingId: item.meeting_id, title: item.meeting_title, date: item.meeting_date, items: [] };
      groups.set(item.meeting_id, group);
    }
    group.items.push(item);
  }
  return [...groups.values()];
}

/** "3 Nov 2026" from the meeting's stored date; the raw text if it does not parse */
export function formatMeetingDate(date: string): string {
  const parsed = new Date(date);
  if (Number.isNaN(parsed.getTime())) return date;
  return parsed.toLocaleDateString(undefined, { day: 'numeric', month: 'short', year: 'numeric' });
}
