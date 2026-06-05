export type JobStatus = "queued" | "processing" | "done" | "error" | "cancelled";

export type ModelStatus = "checking" | "not_setup" | "installing" | "loading_model" | "ready" | "error";

export type MediaType = "wav" | "mp3" | "mp4";

export interface UiFileJob {
  id: string;
  path: string;
  name: string;
  sizeBytes: number;
  durationSec?: number;
  sampleRate?: number;
  mediaType: MediaType;
  validFile: boolean;
  hasAudio: boolean;
  hasVideo: boolean;
  status: JobStatus;
  message?: string;
}

export interface QueueSettings {
  outputDir: string;
  postFilter: boolean;
}

export interface QueueStartPayload {
  jobs: Array<{
    id: string;
    inputPath: string;
    outputDir: string;
    postFilter: boolean;
    mediaType: MediaType;
  }>;
}

export interface ModelStatusPayload {
  status: string;
  message?: string;
  device?: string;
}
