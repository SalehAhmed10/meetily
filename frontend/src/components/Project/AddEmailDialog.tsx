'use client';

import { useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { FileUp, Loader2, Mail } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { parseEmail, type ParsedEmail } from '@/lib/email-parse';
import { dateInputValue } from '@/lib/timeline';

interface AddEmailDialogProps {
  projectId: string;
  projectName: string;
  knownStages: string[];
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onAdded: () => void;
}

const EMPTY: ParsedEmail = { subject: '', from: '', date: null, body: '' };

/** Paste an email or load a saved .eml/.txt; subject, sender and date are read from it */
export function AddEmailDialog({ projectId, projectName, knownStages, open, onOpenChange, onAdded }: AddEmailDialogProps) {
  const [raw, setRaw] = useState('');
  const [parsed, setParsed] = useState<ParsedEmail>(EMPTY);
  const [subject, setSubject] = useState('');
  const [date, setDate] = useState(dateInputValue(new Date()));
  const [stage, setStage] = useState('');
  const [saving, setSaving] = useState(false);
  const fileInput = useRef<HTMLInputElement>(null);

  const reset = () => {
    setRaw('');
    setParsed(EMPTY);
    setSubject('');
    setDate(dateInputValue(new Date()));
    setStage('');
  };

  // Fill subject and date from the email, without overwriting what the user typed
  const read = (text: string) => {
    setRaw(text);
    const email = parseEmail(text);
    setParsed(email);
    if (email.subject && (!subject || subject === parsed.subject)) setSubject(email.subject);
    if (email.date) setDate(dateInputValue(email.date));
  };

  const loadFile = async (file: File | undefined) => {
    if (!file) return;
    try {
      read(await file.text());
    } catch (error) {
      toast.error('Could not read that file', { description: String(error) });
    }
  };

  const title = subject.trim() || parsed.body.split('\n')[0]?.slice(0, 80) || '';
  const canSave = !!parsed.body.trim() && !!title && !!date && !saving;

  const save = async () => {
    if (!canSave) return;
    setSaving(true);
    try {
      // The email's own send time when the date was kept, otherwise midday on the chosen day
      const sentAt = parsed.date && dateInputValue(parsed.date) === date ? parsed.date : new Date(`${date}T12:00:00`);
      const body = parsed.from ? `From: ${parsed.from}\n\n${parsed.body}` : parsed.body;
      await invoke('api_add_project_email', {
        projectId,
        title,
        body,
        sentAt: sentAt.toISOString(),
        stage: stage.trim() || null,
      });
      toast.success(`Email added to ${projectName}`);
      reset();
      onOpenChange(false);
      onAdded();
    } catch (error) {
      toast.error('Could not add the email', { description: String(error) });
    } finally {
      setSaving(false);
    }
  };

  const setOpen = (next: boolean) => {
    if (saving) return;
    onOpenChange(next);
    if (!next) reset();
  };

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogContent className="sm:max-w-xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Mail className="size-5 text-blue-600" aria-hidden /> Add email to {projectName}
          </DialogTitle>
          <DialogDescription>
            Paste the email, or load one saved as .eml or .txt. It is stored on this computer only.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          <div className="space-y-1.5">
            <div className="flex items-center justify-between">
              <Label htmlFor="email-text">Email</Label>
              <Button type="button" variant="ghost" size="sm" onClick={() => fileInput.current?.click()}>
                <FileUp /> Load file
              </Button>
              <input
                ref={fileInput}
                type="file"
                accept=".eml,.txt,message/rfc822,text/plain"
                className="hidden"
                onChange={e => { void loadFile(e.target.files?.[0]); e.target.value = ''; }}
              />
            </div>
            <Textarea
              id="email-text"
              value={raw}
              onChange={e => read(e.target.value)}
              placeholder={'Paste the email here.\n\nTip: copy it with its From / Sent / Subject lines and they are filled in for you.'}
              className="min-h-40 font-[inherit] text-sm"
            />
            {parsed.from && <p className="text-xs text-gray-500">From {parsed.from}</p>}
          </div>

          <div className="grid gap-4 sm:grid-cols-[1fr_auto]">
            <div className="space-y-1.5">
              <Label htmlFor="email-subject">Subject</Label>
              <Input id="email-subject" value={subject} onChange={e => setSubject(e.target.value)} placeholder="e.g. Congratulations, round 2" />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="email-date">Date</Label>
              <Input id="email-date" type="date" value={date} onChange={e => setDate(e.target.value)} />
            </div>
          </div>

          <div className="space-y-1.5">
            <Label htmlFor="email-stage">Stage <span className="font-normal text-gray-500">(optional)</span></Label>
            <Input
              id="email-stage"
              list="email-stage-options"
              value={stage}
              onChange={e => setStage(e.target.value)}
              placeholder={knownStages[knownStages.length - 1] ?? 'e.g. Round 1, Phase 2'}
            />
            <datalist id="email-stage-options">
              {knownStages.map(s => <option key={s} value={s} />)}
            </datalist>
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => setOpen(false)} disabled={saving}>Cancel</Button>
          <Button onClick={() => void save()} disabled={!canSave}>
            {saving && <Loader2 className="animate-spin" />} Add email
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
