import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface JobEventPayload {
  id: string;
  message?: string;
}

export async function bindJobEvents(
  handlers: {
    onStart: (payload: JobEventPayload) => void;
    onProgress: (payload: JobEventPayload) => void;
    onDone: (payload: JobEventPayload) => void;
    onError: (payload: JobEventPayload) => void;
    onCancelled: (payload: JobEventPayload) => void;
  }
): Promise<UnlistenFn[]> {
  const events: Array<[string, (payload: JobEventPayload) => void]> = [
    ["job:start", handlers.onStart],
    ["job:progress", handlers.onProgress],
    ["job:done", handlers.onDone],
    ["job:error", handlers.onError],
    ["job:cancelled", handlers.onCancelled]
  ];

  return Promise.all(
    events.map(async ([eventName, handler]) =>
      listen<JobEventPayload>(eventName, (event) => handler(event.payload))
    )
  );
}
