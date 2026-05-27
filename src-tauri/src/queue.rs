use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::worker::{self, EnhanceCommand};

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueJob {
    pub id: String,
    pub input_path: String,
    pub output_dir: String,
    pub post_filter: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct JobEvent<'a> {
    pub id: &'a str,
    pub message: Option<String>,
}

/// Process all jobs sequentially through the persistent GPU worker.
pub async fn process_queue(
    app: AppHandle,
    worker_state: &worker::WorkerState,
    jobs: Vec<QueueJob>,
    cancel: Arc<AtomicBool>,
) -> Result<(), String> {
    for job in jobs {
        if cancel.load(Ordering::Relaxed) {
            emit_cancelled(&app, &job.id, "Cancelled before start")?;
            continue;
        }

        app.emit(
            "job:start",
            JobEvent {
                id: &job.id,
                message: None,
            },
        )
        .map_err(|e| e.to_string())?;

        // Ensure output directory exists
        let resolved_output = resolve_output_dir(&job.input_path, &job.output_dir)?;
        fs::create_dir_all(&resolved_output).map_err(|e| e.to_string())?;

        // Send the enhance command to the worker
        let cmd = EnhanceCommand {
            cmd: "enhance".into(),
            id: job.id.clone(),
            input: job.input_path.clone(),
            output_dir: resolved_output,
            post_filter: job.post_filter,
        };

        if let Err(e) = worker::send_job(worker_state, &cmd).await {
            let _ = app.emit(
                "job:error",
                JobEvent {
                    id: &job.id,
                    message: Some(e.clone()),
                },
            );
            return Err(e);
        }

        // Wait briefly to give the worker time to process before sending next
        // The actual completion is reported asynchronously via stdout_reader_loop
        // We use a small delay to avoid flooding the worker's stdin
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    Ok(())
}

fn emit_cancelled(app: &AppHandle, id: &str, message: &str) -> Result<(), String> {
    app.emit(
        "job:cancelled",
        JobEvent {
            id,
            message: Some(message.to_string()),
        },
    )
    .map_err(|error| error.to_string())
}

fn resolve_output_dir(input_path: &str, requested_output: &str) -> Result<String, String> {
    if !requested_output.is_empty() {
        return Ok(requested_output.to_string());
    }

    let input = Path::new(input_path);
    let parent = input
        .parent()
        .ok_or_else(|| "Unable to resolve parent directory".to_string())?;
    let output = PathBuf::from(parent).join("enhanced");
    Ok(output.to_string_lossy().to_string())
}
