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
use tauri_plugin_shell::{process::CommandEvent, ShellExt};
use tokio::sync::Semaphore;

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

pub async fn process_queue(
    app: AppHandle,
    jobs: Vec<QueueJob>,
    concurrency: usize,
    cancel: Arc<AtomicBool>,
) -> Result<(), String> {
    let limiter = Arc::new(Semaphore::new(concurrency.max(1)));
    let mut handles = Vec::new();

    for job in jobs {
        let permit = limiter
            .clone()
            .acquire_owned()
            .await
            .map_err(|error| error.to_string())?;
        let app_handle = app.clone();
        let cancel_flag = cancel.clone();
        handles.push(tauri::async_runtime::spawn(async move {
            let _permit_guard = permit;
            run_single_job(app_handle, job, cancel_flag).await
        }));
    }

    for handle in handles {
        let _ = handle.await.map_err(|error| error.to_string())?;
    }
    Ok(())
}

async fn run_single_job(app: AppHandle, job: QueueJob, cancel: Arc<AtomicBool>) -> Result<(), String> {
    if cancel.load(Ordering::Relaxed) {
        emit_cancelled(&app, &job.id, "Cancelled before start")?;
        return Ok(());
    }

    app.emit(
        "job:start",
        JobEvent {
            id: &job.id,
            message: None,
        },
    )
    .map_err(|error| error.to_string())?;

    let resolved_output = resolve_output_dir(&job.input_path, &job.output_dir)?;
    fs::create_dir_all(&resolved_output).map_err(|error| error.to_string())?;
    let mut args = vec!["-o".to_string(), resolved_output];
    if job.post_filter {
        args.push("--pf".to_string());
    }
    args.push(job.input_path.clone());

    let sidecar = app
        .shell()
        .sidecar("deep-filter")
        .map_err(|error| error.to_string())?
        .args(args);

    let (mut rx, child) = sidecar.spawn().map_err(|error| error.to_string())?;
    let mut stderr_lines = Vec::new();

    while let Some(event) = rx.recv().await {
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            emit_cancelled(&app, &job.id, "Cancelled by user")?;
            return Ok(());
        }

        match event {
            CommandEvent::Stderr(line) => {
                let msg = String::from_utf8_lossy(&line).trim().to_string();
                if !msg.is_empty() {
                    stderr_lines.push(msg);
                }
            }
            CommandEvent::Terminated(status) => {
                if status.code == Some(0) {
                    app.emit(
                        "job:done",
                        JobEvent {
                            id: &job.id,
                            message: None,
                        },
                    )
                    .map_err(|error| error.to_string())?;
                } else {
                    let detail = stderr_lines.last().cloned().unwrap_or_else(|| {
                        format!("deep-filter exited with code {:?}", status.code)
                    });
                    app.emit(
                        "job:error",
                        JobEvent {
                            id: &job.id,
                            message: Some(detail),
                        },
                    )
                    .map_err(|error| error.to_string())?;
                }
                break;
            }
            _ => {}
        }
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
