export type JobStatus = "queued" | "processing" | "done" | "error" | "cancelled";

export interface UiFileJob {
  id: string;
  path: string;
  name: string;
  sizeBytes: number;
  durationSec?: number;
  sampleRate?: number;
  validWav: boolean;
  status: JobStatus;
  message?: string;
}

export interface QueueSettings {
  outputDir: string;
  postFilter: boolean;
  concurrency: number;
}

export interface QueueStartPayload {
  jobs: Array<{
    id: string;
    inputPath: string;
    outputDir: string;
    postFilter: boolean;
  }>;
  concurrency: number;
}
