"use client";

import { Suspense, useCallback, useEffect, useMemo, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { AlertCircle, Clock, Folder, ListChecks, LoaderIcon, RefreshCw, User } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useSidebar } from "@/components/Sidebar/SidebarProvider";
import {
  formatMeetingDate,
  groupActionItems,
  type ProjectActionItem,
} from "@/lib/action-items";

type Load =
  | { state: "loading" }
  | { state: "error"; message: string }
  | { state: "ready"; items: ProjectActionItem[] };

function ProjectActionsContent() {
  const projectId = useSearchParams().get("id");
  const router = useRouter();
  const { projects, meetings, setCurrentMeeting } = useSidebar();
  const project = projects.find(p => p.id === projectId);
  const [load, setLoad] = useState<Load>({ state: "loading" });
  const [showDone, setShowDone] = useState(false);

  const fetchItems = useCallback(async () => {
    if (!projectId) return;
    setLoad({ state: "loading" });
    try {
      const items = await invoke<ProjectActionItem[]>("api_get_project_action_items", { projectId });
      setLoad({ state: "ready", items });
    } catch (error) {
      setLoad({ state: "error", message: String(error) });
    }
  }, [projectId]);

  useEffect(() => {
    void fetchItems();
  }, [fetchItems]);

  const items = load.state === "ready" ? load.items : [];
  const groups = useMemo(() => groupActionItems(items, showDone), [items, showDone]);
  const doneCount = items.filter(i => i.done).length;
  const openCount = items.length - doneCount;

  const openMeeting = (id: string, title: string) => {
    setCurrentMeeting({ id, title });
    router.push(`/meeting-details?id=${id}`);
  };

  const setItemDone = (target: ProjectActionItem, done: boolean) =>
    setLoad(prev =>
      prev.state === "ready"
        ? {
            state: "ready",
            items: prev.items.map(i =>
              i.meeting_id === target.meeting_id && i.key === target.key ? { ...i, done } : i,
            ),
          }
        : prev,
    );

  const toggle = async (target: ProjectActionItem) => {
    const done = !target.done;
    setItemDone(target, done);
    try {
      await invoke("api_set_action_item_done", { meetingId: target.meeting_id, key: target.key, done });
    } catch (error) {
      setItemDone(target, !done); // put it back so the list matches what is saved
      toast.error("Could not update the action item", { description: String(error) });
    }
  };

  if (!projectId) {
    return <p className="p-8 text-sm text-gray-500">No project selected.</p>;
  }

  const projectMeetings = meetings.filter(m => m.project_id === projectId);

  return (
    <div className="h-screen overflow-y-auto bg-white">
      <div className="mx-auto max-w-3xl px-6 py-8">
        <header className="mb-6 flex flex-wrap items-start justify-between gap-3">
          <div className="min-w-0">
            <p className="flex items-center gap-1.5 text-sm text-gray-500">
              <Folder className="size-4 text-blue-600" aria-hidden />
              <span className="truncate">{project?.name ?? "Project"}</span>
            </p>
            <h1 className="mt-1 text-2xl font-semibold text-gray-900">Action items</h1>
            {load.state === "ready" && items.length > 0 && (
              <p className="mt-1 text-sm text-gray-500 tabular-nums">
                {openCount} open · {doneCount} done
              </p>
            )}
          </div>
          {load.state === "ready" && doneCount > 0 && (
            <Button variant="outline" size="sm" onClick={() => setShowDone(v => !v)}>
              {showDone ? "Hide done" : `Show done (${doneCount})`}
            </Button>
          )}
        </header>

        {load.state === "loading" && (
          <div className="space-y-3" aria-busy="true" aria-label="Loading action items">
            {[0, 1, 2].map(i => (
              <div key={i} className="h-14 animate-pulse rounded-lg bg-gray-100" />
            ))}
          </div>
        )}

        {load.state === "error" && (
          <div className="flex flex-col items-start gap-3 rounded-lg border border-red-200 bg-red-50 p-4">
            <p className="flex items-center gap-2 text-sm font-medium text-red-700">
              <AlertCircle className="size-4" aria-hidden /> Could not load action items
            </p>
            <p className="text-xs text-red-600 break-all">{load.message}</p>
            <Button variant="outline" size="sm" onClick={() => void fetchItems()}>
              <RefreshCw /> Retry
            </Button>
          </div>
        )}

        {load.state === "ready" && items.length === 0 && (
          <div className="flex flex-col items-center gap-3 rounded-lg border border-dashed border-gray-300 px-6 py-10 text-center">
            <ListChecks className="size-8 text-gray-400" aria-hidden />
            <p className="font-medium text-gray-900">No action items yet</p>
            <p className="max-w-sm text-sm text-gray-500">
              Action items are collected from the summaries of this project&apos;s meetings. Generate a summary on a meeting and its action items show up here.
            </p>
            {projectMeetings[0] && (
              <Button size="sm" onClick={() => openMeeting(projectMeetings[0].id, projectMeetings[0].title)}>
                Open {projectMeetings[0].title}
              </Button>
            )}
          </div>
        )}

        {load.state === "ready" && items.length > 0 && groups.length === 0 && (
          <div className="rounded-lg border border-dashed border-gray-300 px-6 py-10 text-center">
            <p className="font-medium text-gray-900">All caught up</p>
            <p className="mt-1 text-sm text-gray-500">Every action item in this project is done.</p>
          </div>
        )}

        <div className="space-y-6">
          {groups.map(group => (
            <section key={group.meetingId} aria-label={group.title}>
              <button
                type="button"
                onClick={() => openMeeting(group.meetingId, group.title)}
                className="mb-2 flex w-full items-baseline justify-between gap-3 text-left hover:underline"
              >
                <span className="truncate text-sm font-medium text-gray-900">{group.title}</span>
                <span className="shrink-0 text-xs text-gray-500">{formatMeetingDate(group.date)}</span>
              </button>
              <ul className="divide-y divide-gray-100 rounded-lg border border-gray-200">
                {group.items.map(item => (
                  <li key={item.key}>
                    <label className="flex min-h-11 cursor-pointer items-start gap-3 px-3 py-2.5 hover:bg-gray-50">
                      <input
                        type="checkbox"
                        checked={item.done}
                        onChange={() => void toggle(item)}
                        className="mt-0.5 size-4 shrink-0 accent-blue-600"
                      />
                      <span className="min-w-0 flex-1">
                        <span className={`block text-sm ${item.done ? "text-gray-400 line-through" : "text-gray-900"}`}>
                          {item.task}
                        </span>
                        {(item.owner || item.due || item.timestamp) && (
                          <span className="mt-1 flex flex-wrap gap-x-3 gap-y-1 text-xs text-gray-500">
                            {item.owner && (
                              <span className="flex items-center gap-1"><User className="size-3" aria-hidden />{item.owner}</span>
                            )}
                            {item.due && <span>Due {item.due}</span>}
                            {item.timestamp && (
                              <span className="flex items-center gap-1 tabular-nums"><Clock className="size-3" aria-hidden />{item.timestamp}</span>
                            )}
                          </span>
                        )}
                      </span>
                    </label>
                  </li>
                ))}
              </ul>
            </section>
          ))}
        </div>
      </div>
    </div>
  );
}

export default function ProjectActionsPage() {
  return (
    <Suspense
      fallback={
        <div className="flex h-screen items-center justify-center">
          <LoaderIcon className="size-6 animate-spin" />
        </div>
      }
    >
      <ProjectActionsContent />
    </Suspense>
  );
}
