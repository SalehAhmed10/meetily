"use client";

import { Suspense, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { invoke } from "@tauri-apps/api/core";
import { Folder, ListChecks, LoaderIcon, Mail, Rows3 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useSidebar } from "@/components/Sidebar/SidebarProvider";
import { ProjectTimeline, type TimelineLoad } from "@/components/Project/ProjectTimeline";
import { ProjectActionItems } from "@/components/Project/ProjectActionItems";
import { AddEmailDialog } from "@/components/Project/AddEmailDialog";
import { knownStages, type TimelineEntry } from "@/lib/timeline";

type Tab = "timeline" | "actions";

function ProjectContent() {
  const params = useSearchParams();
  const projectId = params.get("id");
  const tab: Tab = params.get("tab") === "actions" ? "actions" : "timeline";
  const router = useRouter();
  const { projects, refetchMeetings } = useSidebar();
  const project = projects.find(p => p.id === projectId);
  const [load, setLoad] = useState<TimelineLoad>({ state: "loading" });
  const [addingEmail, setAddingEmail] = useState(false);
  const latestRequest = useRef(0);

  const fetchTimeline = useCallback(async () => {
    if (!projectId) return;
    // Only the newest request may update the page (switching projects quickly)
    const request = ++latestRequest.current;
    setLoad({ state: "loading" });
    try {
      const entries = await invoke<TimelineEntry[]>("api_get_project_timeline", { projectId });
      if (request === latestRequest.current) setLoad({ state: "ready", entries });
    } catch (error) {
      if (request === latestRequest.current) setLoad({ state: "error", message: String(error) });
    }
  }, [projectId]);

  useEffect(() => {
    void fetchTimeline();
  }, [fetchTimeline]);

  const stages = useMemo(() => (load.state === "ready" ? knownStages(load.entries) : []), [load]);

  const setEntryStage = (entryId: string, stage: string | null) =>
    setLoad(prev =>
      prev.state === "ready"
        ? { state: "ready", entries: prev.entries.map(e => (e.id === entryId ? { ...e, stage } : e)) }
        : prev,
    );

  if (!projectId) {
    return <p className="p-8 text-sm text-gray-500">No project selected.</p>;
  }

  const showTab = (next: string) => router.replace(`/project?id=${encodeURIComponent(projectId)}&tab=${next}`);

  return (
    <div className="h-screen overflow-y-auto bg-white">
      <div className="mx-auto max-w-3xl px-6 py-8">
        <header className="mb-5 flex flex-wrap items-center justify-between gap-3">
          <h1 className="flex min-w-0 items-center gap-2 text-2xl font-semibold text-gray-900">
            <Folder className="size-6 shrink-0 text-blue-600" aria-hidden />
            <span className="truncate">{project?.name ?? "Project"}</span>
          </h1>
          <Button size="sm" variant="outline" onClick={() => setAddingEmail(true)}>
            <Mail /> Add email
          </Button>
        </header>

        <Tabs value={tab} onValueChange={showTab}>
          <TabsList className="mb-5">
            <TabsTrigger value="timeline" className="gap-1.5"><Rows3 className="size-4" aria-hidden /> Timeline</TabsTrigger>
            <TabsTrigger value="actions" className="gap-1.5"><ListChecks className="size-4" aria-hidden /> Action items</TabsTrigger>
          </TabsList>
          <TabsContent value="timeline">
            <ProjectTimeline
              load={load}
              onRetry={() => void fetchTimeline()}
              onStageChanged={setEntryStage}
              onAddEmail={() => setAddingEmail(true)}
            />
          </TabsContent>
          <TabsContent value="actions">
            <ProjectActionItems projectId={projectId} />
          </TabsContent>
        </Tabs>
      </div>

      <AddEmailDialog
        projectId={projectId}
        projectName={project?.name ?? "this project"}
        knownStages={stages}
        open={addingEmail}
        onOpenChange={setAddingEmail}
        onAdded={() => {
          void fetchTimeline();
          void refetchMeetings(); // the email shows in the sidebar folder too
          if (tab !== "timeline") showTab("timeline");
        }}
      />
    </div>
  );
}

export default function ProjectPage() {
  return (
    <Suspense
      fallback={
        <div className="flex h-screen items-center justify-center">
          <LoaderIcon className="size-6 animate-spin" />
        </div>
      }
    >
      <ProjectContent />
    </Suspense>
  );
}
