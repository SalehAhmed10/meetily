'use client';

import { useState } from 'react';
import { Check, Flag, Plus, X } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';

interface StagePickerProps {
  stage: string | null;
  /** Stages already used in this project, offered first */
  known: string[];
  onChange: (stage: string | null) => Promise<void>;
}

/** The stage chip on a timeline row: pick a stage used before, type a new one, or clear it */
export function StagePicker({ stage, known, onChange }: StagePickerProps) {
  const [open, setOpen] = useState(false);
  const [draft, setDraft] = useState('');

  const choose = async (next: string | null) => {
    setOpen(false);
    setDraft('');
    if ((next ?? null) !== stage) await onChange(next);
  };

  return (
    <Popover open={open} onOpenChange={next => { setOpen(next); setDraft(''); }}>
      <PopoverTrigger asChild>
        <button
          type="button"
          onClick={e => e.stopPropagation()}
          className={`inline-flex h-6 max-w-40 shrink-0 items-center gap-1 rounded-full border px-2 text-xs focus-visible:outline focus-visible:outline-2 focus-visible:outline-blue-600 ${
            stage ? 'border-blue-200 bg-blue-50 text-blue-800 hover:bg-blue-100' : 'border-dashed border-gray-300 text-gray-500 hover:border-gray-400 hover:text-gray-700'
          }`}
          aria-label={stage ? `Stage: ${stage}. Change stage` : 'Set stage'}
        >
          <Flag className="size-3 shrink-0" aria-hidden />
          <span className="truncate">{stage ?? 'Set stage'}</span>
        </button>
      </PopoverTrigger>
      <PopoverContent align="end" className="w-60 p-1.5" onClick={e => e.stopPropagation()}>
        {known.length > 0 && (
          <ul className="mb-1.5 max-h-48 overflow-y-auto">
            {known.map(name => (
              <li key={name}>
                <button
                  type="button"
                  onClick={() => void choose(name)}
                  className="flex w-full items-center gap-2 rounded px-2 py-1.5 text-left text-sm hover:bg-gray-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-blue-600"
                >
                  <Check className={`size-3.5 shrink-0 ${name === stage ? 'text-blue-600' : 'invisible'}`} aria-hidden />
                  <span className="truncate">{name}</span>
                </button>
              </li>
            ))}
          </ul>
        )}
        <form
          className="flex gap-1"
          onSubmit={e => { e.preventDefault(); if (draft.trim()) void choose(draft.trim()); }}
        >
          <Input
            autoFocus
            value={draft}
            onChange={e => setDraft(e.target.value)}
            placeholder={known.length ? 'New stage' : 'e.g. Round 1, Phase 2'}
            className="h-8 text-sm"
            aria-label="New stage name"
          />
          <button
            type="submit"
            disabled={!draft.trim()}
            className="flex size-8 shrink-0 items-center justify-center rounded-md border text-gray-600 hover:bg-gray-100 disabled:opacity-40"
            aria-label="Add stage"
          >
            <Plus className="size-4" />
          </button>
        </form>
        {stage && (
          <button
            type="button"
            onClick={() => void choose(null)}
            className="mt-1 flex w-full items-center gap-2 rounded px-2 py-1.5 text-left text-sm text-gray-600 hover:bg-gray-100"
          >
            <X className="size-3.5" aria-hidden /> Clear stage
          </button>
        )}
      </PopoverContent>
    </Popover>
  );
}
