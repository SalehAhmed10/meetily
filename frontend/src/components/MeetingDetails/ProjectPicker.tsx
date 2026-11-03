'use client';

import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { Check, Folder, Pencil, Plus, Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { useSidebar } from '@/components/Sidebar/SidebarProvider';

/** Assign the meeting to a client/project, create one, or rename/delete the current one */
export function ProjectPicker({ meetingId }: { meetingId: string }) {
  const { meetings, projects, refetchMeetings } = useSidebar();
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [newName, setNewName] = useState('');
  const [renameValue, setRenameValue] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);

  const currentId = meetings.find(m => m.id === meetingId)?.project_id ?? null;
  const current = projects.find(p => p.id === currentId) ?? null;

  const handleOpenChange = (next: boolean) => {
    setOpen(next);
    setNewName('');
    setRenameValue(null);
    setConfirmDelete(false);
  };

  /** Run a project change, refresh the sidebar, and report failures; closes on success */
  const run = async (action: () => Promise<unknown>, success?: string) => {
    setBusy(true);
    try {
      await action();
      await refetchMeetings();
      if (success) toast.success(success);
      handleOpenChange(false);
    } catch (error) {
      toast.error('Project update failed', { description: String(error) });
    } finally {
      setBusy(false);
    }
  };

  const assign = (projectId: string | null) =>
    run(() => invoke('api_set_meeting_project', { meetingId, projectId }));

  const create = () => {
    const name = newName.trim();
    if (!name) return;
    return run(async () => {
      const project = await invoke<{ id: string }>('api_create_project', { name });
      await invoke('api_set_meeting_project', { meetingId, projectId: project.id });
    }, `Added to "${name}"`);
  };

  const rename = () => {
    const name = renameValue?.trim();
    if (!current || !name || name === current.name) return;
    return run(() => invoke('api_rename_project', { projectId: current.id, name }), `Renamed to "${name}"`);
  };

  const remove = () => {
    if (!current) return;
    if (!confirmDelete) {
      setConfirmDelete(true);
      return;
    }
    return run(() => invoke('api_delete_project', { projectId: current.id }), `Deleted "${current.name}"`);
  };

  const itemClass = 'w-full flex items-center gap-2 px-2 py-1.5 text-sm rounded-md text-left hover:bg-gray-100 disabled:opacity-50';

  return (
    <Popover open={open} onOpenChange={handleOpenChange}>
      <PopoverTrigger asChild>
        <Button variant="outline" size="sm" title="Project for this meeting" aria-label="Set project">
          <Folder size={16} />
          <span className="max-w-[12rem] truncate">{current?.name ?? 'No project'}</span>
        </Button>
      </PopoverTrigger>
      <PopoverContent align="start" className="w-72 p-2">
        <p className="px-2 pb-1 text-xs font-medium text-gray-500">Project</p>
        <div className="max-h-56 overflow-y-auto">
          <button className={itemClass} disabled={busy} onClick={() => assign(null)}>
            <Check size={14} className={current ? 'invisible' : 'text-blue-600'} />
            <span className="text-gray-600">No project</span>
          </button>
          {projects.map(project => (
            <button key={project.id} className={itemClass} disabled={busy} onClick={() => assign(project.id)}>
              <Check size={14} className={project.id === currentId ? 'text-blue-600' : 'invisible'} />
              <span className="truncate">{project.name}</span>
            </button>
          ))}
        </div>

        <form
          className="mt-2 flex gap-1 border-t border-gray-200 pt-2"
          onSubmit={e => { e.preventDefault(); void create(); }}
        >
          <Input
            value={newName}
            onChange={e => setNewName(e.target.value)}
            placeholder="New project, e.g. client name"
            className="h-8 text-sm"
            disabled={busy}
          />
          <Button type="submit" size="sm" disabled={busy || !newName.trim()} aria-label="Create project">
            <Plus size={16} />
          </Button>
        </form>

        {current && (
          <div className="mt-2 border-t border-gray-200 pt-2 space-y-1">
            {renameValue === null ? (
              <button className={itemClass} disabled={busy} onClick={() => setRenameValue(current.name)}>
                <Pencil size={14} />
                <span className="truncate">Rename &quot;{current.name}&quot;</span>
              </button>
            ) : (
              <form className="flex gap-1" onSubmit={e => { e.preventDefault(); void rename(); }}>
                <Input
                  autoFocus
                  value={renameValue}
                  onChange={e => setRenameValue(e.target.value)}
                  className="h-8 text-sm"
                  disabled={busy}
                />
                <Button type="submit" size="sm" disabled={busy || !renameValue.trim()} aria-label="Save project name">
                  <Check size={16} />
                </Button>
              </form>
            )}
            <button className={`${itemClass} text-red-600 hover:bg-red-50`} disabled={busy} onClick={() => void remove()}>
              <Trash2 size={14} />
              <span>{confirmDelete ? 'Click again to delete (meetings are kept)' : `Delete "${current.name}"`}</span>
            </button>
          </div>
        )}
      </PopoverContent>
    </Popover>
  );
}
