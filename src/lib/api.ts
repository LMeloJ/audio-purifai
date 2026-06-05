import { invoke } from "@tauri-apps/api/core";
import type { QueueStartPayload } from "./types";

export interface MediaInfo {
  durationSec: number;
  sampleRate: number | null;
  channels: number | null;
  mediaType: string;
  hasAudio: boolean;
  hasVideo: boolean;
}

export async function checkEnvironment(): Promise<boolean> {
  return invoke("check_environment");
}

export async function initializeEnvironment(): Promise<void> {
  return invoke("initialize_environment");
}

export async function loadModel(): Promise<void> {
  return invoke("load_model");
}

export async function getModelStatus(): Promise<string> {
  return invoke("get_model_status");
}

export async function probeMedia(path: string): Promise<MediaInfo> {
  return invoke<MediaInfo>("probe_media", { path });
}

export async function checkFfmpeg(): Promise<boolean> {
  return invoke<boolean>("check_ffmpeg");
}

export async function startQueue(payload: QueueStartPayload): Promise<void> {
  return invoke("start_queue", { ...payload });
}

export async function cancelQueue(): Promise<void> {
  return invoke("cancel_queue");
}
