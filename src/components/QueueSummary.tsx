interface QueueSummaryProps {
  done: number;
  processing: number;
  queued: number;
  failed: number;
  total: number;
  running: boolean;
}

export function QueueSummary({ done, processing, queued, failed, total, running }: QueueSummaryProps) {
  if (total === 0) {
    return <p className="text-sm text-slate-400">No files in queue. Add WAV files to get started.</p>;
  }

  const parts: string[] = [];
  if (done > 0) parts.push(`${done} done`);
  if (processing > 0) parts.push(`${processing} processing`);
  if (queued > 0) parts.push(`${queued} queued`);
  if (failed > 0) parts.push(`${failed} failed`);

  return (
    <div className="space-y-1">
      <p className="text-sm text-slate-300">
        <span className="font-medium text-white">
          {done} of {total}
        </span>{" "}
        files complete
        {running ? " · running" : done === total && failed === 0 ? " · all done" : ""}
      </p>
      {parts.length > 0 && <p className="text-xs text-slate-500">{parts.join(" · ")}</p>}
    </div>
  );
}
