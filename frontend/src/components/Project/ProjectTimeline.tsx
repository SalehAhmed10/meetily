'use client';

import { useMemo } from 'react';
import { useRouter } from 'next/navigation';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { AlertCircle, FileText, Mail, Mic, RefreshCw, Sparkles } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useSidebar } from '@/components/Sidebar/SidebarProvider';
import { formatMeetingDate } from '@/lib/action-items';
import { groupByStage, knownStages, type TimelineEntry } from '@/lib/timeline';
import { StagePicker } from './StagePicker';

export type TimelineLoad =
  | { state: 'loading' }
  | { state: 'error'; message: string }
  | { state: 'ready'; entries: TimelineEntry[] };

interface ProjectTimelineProps {
  load: TimelineLoad;
  onRetry: () => void;
  onStageChanged: (entryId: string, stage: string | null) => void;
  onAddEmail: () => void;
}

/** The project's meetings and emails in date order, grouped by stage */
export function ProjectTimeline({ load, onRetry, onStageChanged, onAddEmail }: ProjectTimelineProps) {
  const router = useRouter();
  const { setCurrentMeeting } = useSidebar();
  const entries = useMemo(() => (load.state === 'ready' ? load.entries : []), [load]);
  const groups = useMemo(() => groupByStage(entries), [entries]);
  const stages = useMemo(() => knownStages(entries), [entries]);
  // Group headings only help once something has a stage
  const showHeadings = stages.length > 0;

  const open = (entry: TimelineEntry) => {
    setCurrentMeeting({ id: entry.id, title: entry.title });
    router.push(`/meeting-details?id=${entry.id}`);
  };

  const setStage = async (entry: TimelineEntry, stage: string | null) => {
    onStageChanged(entry.id, stage);
    try {
      await invoke('api_set_meeting_stage', { meetingId: entry.id, stage });
    } catch (error) {
      onStageChanged(entry.id, entry.stage); // put it back
      toast.error('Could not set the stage', { description: String(error) });
    }
  };

  if (load.state === 'loading') {
    return (
      <div className="space-y-2" aria-busy="true" aria-label="Loading timeline">
        {[0, 1, 2].map(i => <div key={i} className="h-12 animate-pulse rounded-lg bg-gray-100" />)}
      </div>
    );
  }

  if (load.state === 'error') {
    return (
      <div className="flex flex-col items-start gap-3 rounded-lg border border-red-200 bg-red-50 p-4">
        <p className="flex items-center gap-2 text-sm font-medium text-red-700">
          <AlertCircle className="size-4" aria-hidden /> Could not load the timeline
        </p>
        <p className="break-all text-xs text-red-600">{load.message}</p>
        <Button variant="outline" size="sm" onClick={onRetry}><RefreshCw /> Retry</Button>
      </div>
    );
  }

  if (entries.length === 0) {
    return (
      <div className="flex flex-col items-center gap-3 rounded-lg border border-dashed border-gray-300 px-6 py-10 text-center">
        <Mail className="size-8 text-gray-400" aria-hidden />
        <p className="font-medium text-gray-900">Nothing in this project yet</p>
        <p className="max-w-sm text-sm text-gray-500">
          Meetings you file into this project and emails you add appear here in date order.
        </p>
        <Button size="sm" onClick={onAddEmail}><Mail /> Add an email</Button>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {!showHeadings && (
        <p className="text-sm text-gray-500">
          Tag meetings and emails with a stage, such as Round 1 or Phase 2, to group them.
        </p>
      )}
      {groups.map(group => (
        <section key={group.stage === null ? 'none' : `stage:${group.stage}`} aria-label={group.stage ?? 'No stage'}>
          {showHeadings && (
            <h2 className="mb-2 flex items-baseline gap-2 text-sm font-semibold text-gray-900">
              {group.stage ?? <span className="font-medium text-gray-500">No stage</span>}
              <span className="text-xs font-normal text-gray-500 tabular-nums">{group.entries.length}</span>
            </h2>
          )}
          <ol className="divide-y divide-gray-100 rounded-lg border border-gray-200">
            {group.entries.map(entry => {
              const Icon = entry.kind === 'email' ? Mail : entry.has_summary ? FileText : Mic;
              return (
                <li key={entry.id} className="flex min-h-12 items-center gap-3 px-3 py-2 hover:bg-gray-50">
                  <Icon className={`size-4 shrink-0 ${entry.kind === 'email' ? 'text-blue-600' : 'text-gray-500'}`} aria-hidden />
                  <button
                    type="button"
                    onClick={() => open(entry)}
                    className="min-w-0 flex-1 rounded text-left focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-600"
                  >
                    <span className="block truncate text-sm text-gray-900 hover:underline underline-offset-4" title={entry.title}>
                      {entry.title}
                    </span>
                    <span className="mt-0.5 flex items-center gap-2 text-xs text-gray-500">
                      <span className="tabular-nums">{formatMeetingDate(entry.date)}</span>
                      <span>{entry.kind === 'email' ? 'Email' : 'Meeting'}</span>
                      {entry.kind !== 'email' && entry.has_summary && (
                        <span className="flex items-center gap-1"><Sparkles className="size-3" aria-hidden />Summary</span>
                      )}
                    </span>
                  </button>
                  <StagePicker stage={entry.stage} known={stages} onChange={stage => setStage(entry, stage)} />
                </li>
              );
            })}
          </ol>
        </section>
      ))}
    </div>
  );
}
