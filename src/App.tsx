import { useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import toast from "react-hot-toast";
import { probeWav, startQueue, cancelQueue, checkEnvironment, initializeEnvironment, loadModel } from "./lib/api";
import { bindJobEvents, listenModelStatus } from "./lib/events";
import type { ModelStatus, QueueSettings, UiFileJob } from "./lib/types";
import { DropZone } from "./components/DropZone";
import { FileRow } from "./components/FileRow";
import { Settings } from "./components/Settings";
import { QueueSummary } from "./components/QueueSummary";
import { TitleBar } from "./components/TitleBar";
import { Cpu } from "lucide-react";

function basename(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

let initialCheckStarted = false;

export default function App() {
  const [files, setFiles] = useState<UiFileJob[]>([]);
  const [isDragging, setIsDragging] = useState(false);
  const [running, setRunning] = useState(false);
  const [settings, setSettings] = useState<QueueSettings>({
    outputDir: "",
    postFilter: false
  });
  const [modelStatus, setModelStatus] = useState<ModelStatus>("checking");
  const [modelDevice, setModelDevice] = useState<string>("");
  const [envError, setEnvError] = useState("");

  // Check environment on mount, then auto-load model if env is ready
  useEffect(() => {
    if (initialCheckStarted) return;
    initialCheckStarted = true;

    checkEnvironment()
      .then((ready) => {
        if (ready) {
          setModelStatus("loading_model");
          loadModel().catch((e) => {
            setModelStatus("error");
            setEnvError(String(e));
          });
        } else {
          setModelStatus("not_setup");
        }
      })
      .catch(() => setModelStatus("not_setup"));
  }, []);

  // Listen for model status events from the Rust backend
  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | null = null;

    listenModelStatus((payload) => {
      if (disposed) return;
      switch (payload.status) {
        case "loading":
          setModelStatus("loading_model");
          break;
        case "ready":
          setModelStatus("ready");
          if (payload.device) setModelDevice(payload.device);
          break;
        case "error":
          setModelStatus("error");
          setEnvError(payload.message ?? "Unknown error");
          break;
      }
    }).then((fn) => {
      if (disposed) fn();
      else unlisten = fn;
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  // Bind job events
  useEffect(() => {
    let disposed = false;
    let unlisteners: Array<() => void> = [];

    bindJobEvents({
      onStart: ({ id }) =>
        setFiles((items) => items.map((item) => (item.id === id ? { ...item, status: "processing", message: undefined } : item))),
      onProgress: ({ id, message }) =>
        setFiles((items) => items.map((item) => (item.id === id ? { ...item, message } : item))),
      onDone: ({ id }) =>
        setFiles((items) => items.map((item) => (item.id === id ? { ...item, status: "done", message: undefined } : item))),
      onError: ({ id, message }) =>
        setFiles((items) => items.map((item) => (item.id === id ? { ...item, status: "error", message } : item))),
      onCancelled: ({ id, message }) =>
        setFiles((items) =>
          items.map((item) => (item.id === id ? { ...item, status: "cancelled", message: message ?? "Cancelled" } : item))
        )
    }).then((listeners) => {
      if (disposed) {
        listeners.forEach((u) => u());
      } else {
        unlisteners = listeners;
      }
    });

    const unlisten = getCurrentWebview().onDragDropEvent(async (event) => {
      if (event.payload.type === "over") {
        setIsDragging(true);
      } else if (event.payload.type === "drop") {
        setIsDragging(false);
        await addPaths(event.payload.paths);
      } else {
        setIsDragging(false);
      }
    });

    return () => {
      disposed = true;
      unlisteners.forEach((u) => u());
      Promise.resolve(unlisten).then((fn) => fn());
    };
  }, []);

  const queueStats = useMemo(() => {
    const done = files.filter((f) => f.status === "done").length;
    const processing = files.filter((f) => f.status === "processing").length;
    const queued = files.filter((f) => f.status === "queued").length;
    const failed = files.filter((f) => f.status === "error" || f.status === "cancelled").length;
    const validQueued = files.filter((f) => f.status === "queued" && f.validWav).length;
    return { done, processing, queued, failed, total: files.length, validQueued };
  }, [files]);

  useEffect(() => {
    if (!running) {
      return;
    }
    const remaining = files.some((file) => file.status === "queued" || file.status === "processing");
    if (!remaining) {
      setRunning(false);
      const { done, failed, total } = queueStats;
      if (total > 0 && done + failed === total) {
        toast.success(failed > 0 ? `Finished: ${done} succeeded, ${failed} failed or cancelled` : `All ${done} files enhanced.`);
      }
    }
  }, [files, running, queueStats]);

  async function addPaths(paths: string[]) {
    const wavPaths = paths.filter((path) => path.toLowerCase().endsWith(".wav"));
    if (wavPaths.length !== paths.length) {
      toast.error("Only WAV files are supported.");
    }
    const additions: UiFileJob[] = [];
    for (const path of wavPaths) {
      const id = crypto.randomUUID();
      try {
        const info = await probeWav(path);
        const valid = info.sampleRate === 48_000;
        additions.push({
          id,
          path,
          name: basename(path),
          sizeBytes: 0,
          durationSec: info.durationSec,
          sampleRate: info.sampleRate,
          validWav: valid,
          status: "queued",
          message: valid ? undefined : "Expected 48kHz WAV"
        });
      } catch {
        additions.push({
          id,
          path,
          name: basename(path),
          sizeBytes: 0,
          validWav: false,
          status: "error",
          message: "Invalid WAV file"
        });
      }
    }
    setFiles((existing) => [...existing, ...additions]);
  }

  async function chooseFiles() {
    const selected = await open({ multiple: true, filters: [{ name: "WAV Audio", extensions: ["wav"] }] });
    if (!selected) {
      return;
    }
    const paths = Array.isArray(selected) ? selected : [selected];
    await addPaths(paths);
  }

  async function chooseOutputDir() {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string") {
      setSettings((s) => ({ ...s, outputDir: selected }));
    }
  }

  function clearCompleted() {
    setFiles((items) => items.filter((item) => item.status !== "done"));
  }

  async function onStart() {
    const jobs = files.filter((file) => file.status === "queued" && file.validWav).map((file) => ({
      id: file.id,
      inputPath: file.path,
      outputDir: settings.outputDir,
      postFilter: settings.postFilter
    }));
    if (jobs.length === 0) {
      toast.error("No valid queued files to process.");
      return;
    }
    setRunning(true);
    try {
      await startQueue({ jobs });
    } catch (error) {
      toast.error(`Failed to start queue: ${String(error)}`);
      setRunning(false);
    }
  }

  async function onCancel() {
    await cancelQueue();
    setRunning(false);
    toast("Cancellation requested.");
  }

  async function handleInitialize() {
    setModelStatus("installing");
    try {
      await initializeEnvironment();
      // Environment ready, now load the model
      setModelStatus("loading_model");
      await loadModel();
    } catch (e: any) {
      setModelStatus("error");
      setEnvError(String(e));
    }
  }

  async function handleRetryModel() {
    setModelStatus("loading_model");
    setEnvError("");
    try {
      await loadModel();
    } catch (e: any) {
      setModelStatus("error");
      setEnvError(String(e));
    }
  }

  const canStart = !running && queueStats.validQueued > 0;
  const hasCompleted = files.some((f) => f.status === "done");

  // ── Setup / Loading screens ────────────────────────────────────────────
  if (modelStatus !== "ready") {
    return (
      <div className="flex h-screen flex-col overflow-hidden bg-[#09090b] text-slate-200">
        <TitleBar />
        <div className="flex flex-1 items-center justify-center">
          <div className="max-w-md w-full rounded-2xl border border-white/5 bg-zinc-900/40 p-8 shadow-2xl backdrop-blur-xl text-center">
          <h2 className="text-2xl font-bold mb-2">
            {["loading_model", "checking"].includes(modelStatus) ? "Loading Model" : "Setup Required"}
          </h2>
          <p className="text-zinc-400 mb-6">
            {["loading_model", "checking"].includes(modelStatus)
              ? "Loading DeepFilterNet3 into GPU memory. This takes a moment on first launch…"
              : "To use GPU-accelerated audio processing, we need to initialize a local Python environment with PyTorch and DeepFilterNet."}
          </p>
          {modelStatus === "checking" && <p className="animate-pulse text-cyan-400">Verifying environment…</p>}
          {modelStatus === "not_setup" && (
            <button
              className="w-full rounded-lg bg-gradient-to-r from-purple-600 to-cyan-500 py-3 font-semibold shadow-lg hover:scale-105 transition-all"
              onClick={handleInitialize}
            >
              Initialize Environment (Takes a few minutes)
            </button>
          )}
          {modelStatus === "installing" && (
            <div className="flex flex-col gap-3">
              <div className="w-full bg-zinc-800 rounded-full h-2 overflow-hidden relative">
                <div className="absolute inset-0 bg-gradient-to-r from-purple-500 to-cyan-500 animate-[translate_2s_linear_infinite] w-1/2"></div>
              </div>
              <p className="text-sm text-cyan-400 animate-pulse">Installing PyTorch &amp; DeepFilterNet…</p>
            </div>
          )}
          {modelStatus === "loading_model" && (
            <div className="flex flex-col gap-3 items-center">
              <div className="relative w-16 h-16">
                <Cpu className="w-16 h-16 text-cyan-400 animate-pulse" />
                <div className="absolute inset-0 rounded-full bg-cyan-400/10 animate-ping" />
              </div>
              <p className="text-sm text-cyan-400 animate-pulse">Loading model into GPU…</p>
            </div>
          )}
          {modelStatus === "error" && (
            <div className="text-left bg-rose-950/40 border border-rose-500/30 p-4 rounded-lg mt-4">
              <p className="text-rose-400 font-bold text-sm mb-1">Failed</p>
              <pre className="text-xs text-rose-300 whitespace-pre-wrap overflow-auto max-h-48">{envError}</pre>
              <div className="mt-3 flex gap-3">
                <button onClick={() => setModelStatus("not_setup")} className="text-sm underline text-zinc-400 hover:text-white">Re-install</button>
                <button onClick={handleRetryModel} className="text-sm underline text-cyan-400 hover:text-white">Retry Model Load</button>
              </div>
            </div>
          )}
          </div>
        </div>
      </div>
    );
  }

  // ── Main app ───────────────────────────────────────────────────────────
  return (
    <div className="flex h-screen flex-col overflow-hidden bg-[#09090b] text-slate-200">
      <TitleBar />
      <main className="mx-auto flex h-full w-full max-w-5xl flex-col gap-6 overflow-auto p-6 scrollbar-thin relative z-10">
        <header>
          <h1 className="text-4xl font-bold tracking-tight text-white flex items-center gap-3">
            <span className="bg-gradient-to-r from-cyan-300 to-purple-400 bg-clip-text text-transparent">Bulk Audio PurifAI</span>
          </h1>
          <div className="flex items-center gap-3 mt-2">
            <QueueSummary
              done={queueStats.done}
              processing={queueStats.processing}
              queued={queueStats.queued}
              failed={queueStats.failed}
              total={queueStats.total}
              running={running}
            />
            {modelDevice && (
              <span className="inline-flex items-center gap-1.5 rounded-full border border-teal-500/30 bg-teal-950/30 px-3 py-1 text-[11px] font-semibold uppercase tracking-wider text-teal-300">
                <Cpu className="w-3 h-3" />
                {modelDevice}
              </span>
            )}
          </div>
        </header>

      <div className="relative rounded-2xl border border-white/5 bg-zinc-900/40 p-5 shadow-2xl backdrop-blur-xl">
        <DropZone isDragging={isDragging} />
        <div className="mb-6 flex flex-wrap gap-3">
          <button
            className="flex items-center gap-2 rounded-lg border border-cyan-500/30 bg-cyan-950/40 px-4 py-2.5 text-sm font-medium text-cyan-300 transition-all hover:bg-cyan-900/60 hover:shadow-[0_0_15px_rgba(34,211,238,0.2)] disabled:opacity-50"
            onClick={chooseFiles}
            disabled={running}
          >
            Add WAV files
          </button>
          <button
            className="group flex items-center gap-2 rounded-lg bg-gradient-to-r from-purple-600 to-cyan-500 px-5 py-2.5 text-sm font-semibold text-white shadow-[0_0_20px_rgba(168,85,247,0.4)] transition-all hover:scale-105 hover:shadow-[0_0_30px_rgba(168,85,247,0.6)] disabled:opacity-50 disabled:hover:scale-100"
            onClick={onStart}
            disabled={!canStart}
          >
            Start Purifying
            <span className="transition-transform group-hover:translate-x-1">▶</span>
          </button>
          <button
            className="flex items-center gap-2 rounded-lg border border-rose-500/30 bg-rose-950/40 px-4 py-2.5 text-sm font-medium text-rose-300 transition-all hover:bg-rose-900/60 hover:shadow-[0_0_15px_rgba(244,63,94,0.2)] disabled:opacity-50"
            onClick={onCancel}
            disabled={!running}
          >
            Cancel All ✕
          </button>
          <button
            className="flex items-center gap-2 rounded-lg border border-purple-500/30 bg-purple-950/40 px-4 py-2.5 text-sm font-medium text-purple-300 transition-all hover:bg-purple-900/60 hover:shadow-[0_0_15px_rgba(168,85,247,0.2)] disabled:opacity-50"
            onClick={clearCompleted}
            disabled={!hasCompleted || running}
          >
            Clear completed
          </button>
        </div>
        <Settings
          settings={settings}
          onChooseOutput={chooseOutputDir}
          onTogglePostFilter={() => setSettings((s) => ({ ...s, postFilter: !s.postFilter }))}
        />
      </div>

      <section className="grid gap-3 overflow-auto pb-3 flex-1 content-start">
        {files.length === 0 ? (
          <p className="rounded-xl border border-dashed border-white/10 bg-white/5 py-12 text-center text-sm text-zinc-500">
            Drop WAV files here or click Add WAV files
          </p>
        ) : (
          files.map((file) => (
            <FileRow key={file.id} file={file} onRemove={(id) => setFiles((items) => items.filter((item) => item.id !== id))} />
          ))
        )}
      </section>

      </main>
    </div>
  );
}
