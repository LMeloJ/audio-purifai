interface DropZoneProps {
  isDragging: boolean;
}

export function DropZone({ isDragging }: DropZoneProps) {
  return (
    <div
      className={`pointer-events-none absolute inset-0 rounded-2xl border-2 border-dashed transition-all z-20 ${
        isDragging ? "border-cyan-400 bg-cyan-950/40 opacity-100 shadow-[inset_0_0_30px_rgba(34,211,238,0.2)]" : "border-transparent opacity-0"
      }`}
    >
      <div className="flex h-full items-center justify-center text-xl font-bold tracking-wider text-cyan-300 text-glow">
        DROP WAV FILES HERE
      </div>
    </div>
  );
}
