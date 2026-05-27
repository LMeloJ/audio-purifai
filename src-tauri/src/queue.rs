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

    let bin_dir = crate::commands::get_venv_bin_dir(&app).map_err(|e| e.to_string())?;
    let deep_filter_exe = if cfg!(target_os = "windows") {
        bin_dir.join("deepFilter.exe")
    } else {
        bin_dir.join("deepFilter")
    };

    let mut child = match tokio::process::Command::new(deep_filter_exe)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            let detail = if error.kind() == std::io::ErrorKind::NotFound {
                "deepFilter not found. Please click 'Initialize Environment' first.".to_string()
            } else {
                error.to_string()
            };
            let _ = app.emit(
                "job:error",
                JobEvent {
                    id: &job.id,
                    message: Some(detail.clone()),
                },
            );
            return Err(detail);
        }
    };

    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;

    use tokio::io::AsyncBufReadExt;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    // Spawn a task to read stdout lines
    let tx_out = tx.clone();
    tokio::spawn(async move {
        let mut reader = tokio::io::BufReader::new(stdout);
        let mut line = String::new();
        loop {
            match reader.read_line(&mut line).await {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let msg = line.trim().to_string();
                    if !msg.is_empty() {
                        let _ = tx_out.send(msg);
                    }
                    line.clear();
                }
            }
        }
    });

    // Spawn a task to read stderr lines
    let tx_err = tx;
    tokio::spawn(async move {
        let mut reader = tokio::io::BufReader::new(stderr);
        let mut line = String::new();
        loop {
            match reader.read_line(&mut line).await {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let msg = line.trim().to_string();
                    if !msg.is_empty() {
                        let _ = tx_err.send(msg);
                    }
                    line.clear();
                }
            }
        }
    });

    // Drop our reference so rx closes when both tasks finish
    let mut output_lines = Vec::new();

    loop {
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill().await;
            emit_cancelled(&app, &job.id, "Cancelled by user")?;
            return Ok(());
        }

        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Some(msg) => {
                        println!("[deepFilter] {}", msg);
                        output_lines.push(msg.clone());
                        let _ = app.emit("job:progress", JobEvent { id: &job.id, message: Some(msg) });
                    }
                    None => break, // Both stdout and stderr closed
                }
            }
            status = child.wait() => {
                // Drain remaining messages
                while let Ok(msg) = rx.try_recv() {
                    println!("[deepFilter] {}", msg);
                    output_lines.push(msg.clone());
                    let _ = app.emit("job:progress", JobEvent { id: &job.id, message: Some(msg) });
                }
                if let Ok(status) = status {
                    if status.success() {
                        app.emit("job:done", JobEvent { id: &job.id, message: None })
                            .map_err(|error| error.to_string())?;
                    } else {
                        let detail = output_lines.last().cloned().unwrap_or_else(|| {
                            format!("deep-filter exited with code {:?}", status.code())
                        });
                        app.emit("job:error", JobEvent { id: &job.id, message: Some(detail) })
                            .map_err(|error| error.to_string())?;
                    }
                }
                return Ok(());
            }
        }
    }

    // Fallback if channel closed before wait finishes
    let status = child.wait().await.map_err(|e| e.to_string())?;
    if status.success() {
        app.emit("job:done", JobEvent { id: &job.id, message: None })
            .map_err(|error| error.to_string())?;
    } else {
        let detail = output_lines.last().cloned().unwrap_or_else(|| {
            format!("deep-filter exited with code {:?}", status.code())
        });
        app.emit("job:error", JobEvent { id: &job.id, message: Some(detail) })
            .map_err(|error| error.to_string())?;
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
