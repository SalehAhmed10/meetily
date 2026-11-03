"use client";

import { useEffect, useRef, useState } from 'react';
import type { SummaryProgress } from '@/types';

/** Used until there is a finished summary to learn the typical length from */
const FALLBACK_EXPECTED_CHARS = 3000;
/** The model decides when it is done, so the estimate never claims 100% early */
const MAX_ESTIMATED_PERCENT = 95;

export function formatDuration(ms: number): string {
  const seconds = Math.max(0, Math.round(ms / 1000));
  if (seconds < 60) return `${seconds}s`;
  return `${Math.floor(seconds / 60)}m ${String(seconds % 60).padStart(2, '0')}s`;
}

type Sample = { at: number; chars: number };

/** Seconds left at the speed measured between two progress samples, counted down from the latest */
export function estimateSecondsLeft(first: Sample, last: Sample, expectedChars: number, now: number): number | null {
  if (last.at <= first.at || last.chars <= first.chars) return null;
  const charsPerMs = (last.chars - first.chars) / (last.at - first.at);
  const msLeftAtLastSample = Math.max(0, expectedChars - last.chars) / charsPerMs;
  return Math.ceil((msLeftAtLastSample - (now - last.at)) / 1000);
}

interface SummaryProgressViewProps {
  progress: SummaryProgress | null;
  startedAt: string | null;
}

export function SummaryProgressView({ progress, startedAt }: SummaryProgressViewProps) {
  const [now, setNow] = useState(() => Date.now());
  const mountedAtRef = useRef(Date.now());
  // First and latest progress samples while writing; speed comes from these, not the 1s clock tick
  const firstSampleRef = useRef<Sample | null>(null);
  const lastSampleRef = useRef<Sample | null>(null);

  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, []);

  const startMs = startedAt ? Date.parse(startedAt) : NaN;
  const elapsedMs = now - (Number.isFinite(startMs) ? startMs : mountedAtRef.current);

  const chars = progress?.generated_chars ?? 0;
  const isWriting = !!progress && progress.generated_tokens > 0;
  if (isWriting && lastSampleRef.current?.chars !== chars) {
    const sample = { at: Date.now(), chars };
    firstSampleRef.current ??= sample;
    lastSampleRef.current = sample;
  }

  const expectedChars = progress?.expected_chars || FALLBACK_EXPECTED_CHARS;
  const percent = isWriting ? Math.min(MAX_ESTIMATED_PERCENT, Math.round((chars / expectedChars) * 100)) : 0;

  const first = firstSampleRef.current;
  const last = lastSampleRef.current;
  const secondsLeft = isWriting && first && last ? estimateSecondsLeft(first, last, expectedChars, now) : null;

  let stage = 'Generating summary...';
  if (progress && !isWriting) {
    stage = `Reading transcript (${progress.prompt_tokens.toLocaleString()} tokens)...`;
  } else if (isWriting) {
    stage = 'Writing summary...';
  }

  const details = [
    isWriting ? `${chars.toLocaleString()} characters` : null,
    `${formatDuration(elapsedMs)} elapsed`,
    secondsLeft !== null && secondsLeft > 0 ? `about ${formatDuration(secondsLeft * 1000)} left` : null,
  ].filter(Boolean).join(' · ');

  return (
    <div className="w-full max-w-md px-6 text-center">
      <div className="flex items-baseline justify-between mb-2">
        <p className="text-gray-700 font-medium">{stage}</p>
        {isWriting && <p className="text-sm text-gray-500 tabular-nums">~{percent}%</p>}
      </div>
      <div
        className="h-2 w-full rounded-full bg-gray-200 overflow-hidden"
        role="progressbar"
        aria-label="Summary progress"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={isWriting ? percent : undefined}
      >
        {isWriting ? (
          <div className="h-full bg-blue-500 transition-[width] duration-700 ease-out" style={{ width: `${percent}%` }} />
        ) : (
          <div className="h-full w-1/3 bg-blue-300 animate-pulse" />
        )}
      </div>
      <p className="mt-2 text-xs text-gray-500 tabular-nums">{details}</p>
    </div>
  );
}
