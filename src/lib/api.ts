import { invoke } from "@tauri-apps/api/core";
import type { QueueStartPayload } from "./types";

export interface WavInfo {
  durationSec: number;
  sampleRate: number;
  channels: number;
}

export async function probeWav(path: string): Promise<WavInfo> {
  return invoke<WavInfo>("probe_wav", { path });
}

export async function startQueue(payload: QueueStartPayload): Promise<void> {
  return invoke("start_queue", {payload});
}

export async function cancelQueue(): Promise<void> {
  return invoke("cancel_queue");
}
