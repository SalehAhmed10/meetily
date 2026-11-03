/** One meeting or email on a project's timeline, as api_get_project_timeline returns it */
export interface TimelineEntry {
  id: string;
  title: string;
  kind: 'meeting' | 'email' | string;
  stage: string | null;
  date: string;
  has_summary: boolean;
}

export interface StageGroup {
  /** null: entries with no stage yet */
  stage: string | null;
  entries: TimelineEntry[];
}

const stageKey = (stage: string | null) => stage?.trim().toLowerCase() || null;

/**
 * Entries (oldest first) grouped by stage, groups in the order their first entry happened, so
 * "Round 1" comes before "Round 2". Stage names match ignoring case; the first spelling is shown.
 */
export function groupByStage(entries: TimelineEntry[]): StageGroup[] {
  const groups = new Map<string | null, StageGroup>();
  for (const entry of entries) {
    const key = stageKey(entry.stage);
    let group = groups.get(key);
    if (!group) {
      group = { stage: key === null ? null : entry.stage!.trim(), entries: [] };
      groups.set(key, group);
    }
    group.entries.push(entry);
  }
  return [...groups.values()];
}

/** Stage names already used in the project, in timeline order, for the stage picker */
export function knownStages(entries: TimelineEntry[]): string[] {
  return groupByStage(entries).flatMap(g => (g.stage ? [g.stage] : []));
}

/** The value for a date input: local yyyy-mm-dd */
export function dateInputValue(date: Date): string {
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}
