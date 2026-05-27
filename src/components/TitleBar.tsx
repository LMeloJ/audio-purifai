import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Square, X } from "lucide-react";
import { useState, useEffect } from "react";
import toast from "react-hot-toast";
// @ts-ignore - Vite handles png imports but tsc doesn't know about it
import logoUrl from "../../images/logo-icon-square.png";

export function TitleBar() {
  const [isMaximized, setIsMaximized] = useState(false);
  const appWindow = getCurrentWindow();

  useEffect(() => {
    let unlisten: () => void;
    appWindow.onResized(async () => {
      try {
        setIsMaximized(await appWindow.isMaximized());
      } catch (e) {}
    }).then((fn: () => void) => {
      unlisten = fn;
    }).catch(err => console.error("onResized error", err));
    return () => {
      if (unlisten) unlisten();
    };
  }, [appWindow]);

  const handleAction = async (action: 'minimize' | 'maximize' | 'close') => {
    try {
      if (action === 'minimize') await appWindow.minimize();
      else if (action === 'maximize') await appWindow.toggleMaximize();
      else if (action === 'close') await appWindow.close();
    } catch (err) {
      toast.error(`Window ${action} failed: ${err}`);
    }
  };

  return (
    <div
      onPointerDown={(e) => {
        // Only start dragging if we didn't click a button inside
        if (!(e.target as HTMLElement).closest('button')) {
          appWindow.startDragging().catch(err => toast.error(`Drag failed: ${err}`));
        }
      }}
      className="flex h-10 shrink-0 select-none items-center justify-between bg-zinc-950/80 px-4 backdrop-blur-md border-b border-white/5"
    >
      {/* Brand */}
      <div className="flex items-center gap-2 pointer-events-none">
        <img src={logoUrl} className="h-6 w-6 rounded shadow-[0_0_10px_rgba(168,85,247,0.4)]" alt="Logo" />
        <span className="text-sm font-medium tracking-wide text-zinc-100">
          <span className="text-zinc-400">Audio</span> PurifAI
        </span>
      </div>

      {/* Center title optional */}
      <div className="pointer-events-none text-xs text-zinc-500">Audio PurifAI</div>

      {/* Controls */}
      <div className="flex items-center gap-1">
        <button
          className="inline-flex h-7 w-7 items-center justify-center rounded text-zinc-400 hover:bg-white/10 hover:text-zinc-100 transition-colors"
          onClick={() => handleAction('minimize')}
          title="Minimize"
        >
          <Minus className="h-4 w-4" />
        </button>
        <button
          className="inline-flex h-7 w-7 items-center justify-center rounded text-zinc-400 hover:bg-white/10 hover:text-zinc-100 transition-colors"
          onClick={() => handleAction('maximize')}
          title="Maximize"
        >
          <Square className="h-3 w-3" />
        </button>
        <button
          className="inline-flex h-7 w-7 items-center justify-center rounded text-zinc-400 hover:bg-red-500/20 hover:text-red-400 transition-colors"
          onClick={() => handleAction('close')}
          title="Close"
        >
          <X className="h-4 w-4" />
        </button>
      </div>
    </div>
  );
}
