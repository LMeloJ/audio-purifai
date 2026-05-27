import type { QueueSettings } from "../lib/types";

interface SettingsProps {
  settings: QueueSettings;
  onChooseOutput: () => void;
  onTogglePostFilter: () => void;
}

export function Settings({
  settings,
  onChooseOutput,
  onTogglePostFilter
}: SettingsProps) {
  return (
    <div className="grid grid-cols-1 gap-4 rounded-xl border border-white/5 bg-black/20 p-5 md:grid-cols-2 items-end">
      <div>
        <p className="mb-2 text-xs font-semibold tracking-wider text-zinc-500 uppercase">Output directory</p>
        <button
          type="button"
          onClick={onChooseOutput}
          className="w-full truncate rounded-lg border border-purple-500/40 bg-purple-950/20 px-4 py-2.5 text-left text-sm text-purple-200 transition-all hover:bg-purple-900/30 hover:shadow-[0_0_15px_rgba(168,85,247,0.3)] focus:outline-none focus:ring-2 focus:ring-purple-500"
        >
          {settings.outputDir || "Choose folder..."}
        </button>
      </div>
      <div className="flex h-[42px] items-center rounded-lg border border-white/5 bg-zinc-900/50 px-4">
        <label className="flex w-full cursor-pointer items-center justify-between gap-3 text-sm text-zinc-300">
          <span>Post-filter (`--pf`)</span>
          <div className="relative inline-flex h-6 w-11 items-center rounded-full transition-colors" style={{ backgroundColor: settings.postFilter ? "rgb(6, 182, 212)" : "rgba(255,255,255,0.1)" }}>
            <input type="checkbox" className="sr-only" checked={settings.postFilter} onChange={onTogglePostFilter} />
            <span
              className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
                settings.postFilter ? "translate-x-6 shadow-[0_0_10px_rgba(255,255,255,0.8)]" : "translate-x-1"
              }`}
            />
          </div>
        </label>
      </div>
    </div>
  );
}
