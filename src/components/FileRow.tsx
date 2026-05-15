import type { JobStatus, UiFileJob } from "../lib/types";
import { formatDuration } from "../lib/format";

interface FileRowProps {
  file: UiFileJob;
  onRemove: (id: string) => void;
}

const statusLabel: Record<JobStatus, string> = {
  queued: "Queued",
  processing: "Enhancing",
  done: "Done",
  error: "Failed",
  cancelled: "Cancelled"
};

const statusClass: Record<JobStatus, string> = {
  queued: "bg-zinc-800 text-zinc-300 border border-white/10",
  processing: "bg-cyan-950 border border-cyan-500/50 text-cyan-300 shadow-[0_0_10px_rgba(34,211,238,0.2)]",
  done: "bg-teal-950 border border-teal-500/50 text-teal-300 shadow-[0_0_10px_rgba(20,184,166,0.2)]",
  error: "bg-rose-950 border border-rose-500/50 text-rose-300",
  cancelled: "bg-amber-950 border border-amber-500/50 text-amber-300"
};

export function FileRow({ file, onRemove }: FileRowProps) {
  return (
    <div className="group rounded-xl border border-white/5 bg-zinc-900/40 p-4 transition-all hover:bg-zinc-800/60 hover:border-white/10">
      <div className="flex items-center justify-between gap-4">
        <div className="min-w-0 flex-1">
          <p className="truncate text-sm font-medium text-cyan-100/90">{file.name}</p>
          <p className="mt-1 text-[11px] font-medium tracking-wide text-zinc-500">
            {file.sampleRate ? `${file.sampleRate} Hz` : "Unknown rate"}
            {file.durationSec !== undefined ? ` · ${formatDuration(file.durationSec)}` : ""}
          </p>
          {file.message && <p className="mt-1 text-xs text-rose-400">{file.message}</p>}
        </div>
        <div className="flex shrink-0 items-center gap-3">
          <span
            className={`inline-flex items-center gap-2 rounded-full px-3 py-1 text-[11px] font-semibold uppercase tracking-wider ${statusClass[file.status]}`}
          >
            {file.status === "processing" && (
              <span className="inline-block h-3 w-3 animate-spin rounded-full border-2 border-cyan-300 border-t-transparent shadow-[0_0_8px_rgba(34,211,238,0.5)]" />
            )}
            {statusLabel[file.status]}
          </span>
          <button
            type="button"
            onClick={() => onRemove(file.id)}
            disabled={file.status === "processing"}
            className="flex items-center gap-2 rounded-lg border border-white/10 bg-black/20 px-3 py-1 text-[11px] font-semibold tracking-wider text-zinc-400 transition-colors hover:border-rose-500/50 hover:text-rose-300 disabled:cursor-not-allowed disabled:opacity-40 uppercase"
          >
            <span>✕</span> <span className="opacity-50">|</span> Remove
          </button>
        </div>
      </div>
    </div>
  );
}
