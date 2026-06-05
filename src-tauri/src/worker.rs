use std::path::PathBuf;
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin};
use tokio::sync::Mutex;

use crate::queue::JobEvent;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkerResponse {
    #[serde(default)]
    pub id: Option<String>,
    pub status: String,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub device: Option<String>,
    #[serde(default)]
    pub output: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ModelStatusEvent {
    pub status: String,
    pub message: Option<String>,
    pub device: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnhanceCommand {
    pub cmd: String,
    pub id: String,
    pub input: String,
    pub output_dir: String,
    pub post_filter: bool,
    pub media_type: String,
}

/// Shared state for the persistent worker process.
pub struct WorkerState {
    inner: Mutex<Option<WorkerInner>>,
}

struct WorkerInner {
    stdin: ChildStdin,
    child: Child,
}

impl Default for WorkerState {
    fn default() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }
}

// ---------------------------------------------------------------------------
// Worker lifecycle
// ---------------------------------------------------------------------------

/// Resolve the Python executable path inside the app's venv.
fn python_exe(app: &AppHandle) -> Result<PathBuf, String> {
    let bin_dir = crate::commands::get_venv_bin_dir(app)?;
    let exe = if cfg!(target_os = "windows") {
        bin_dir.join("python.exe")
    } else {
        bin_dir.join("python")
    };
    if !exe.exists() {
        return Err(format!(
            "Python not found at {}. Please initialize the environment first.",
            exe.display()
        ));
    }
    Ok(exe)
}

/// Resolve the worker.py script path (bundled as a resource).
fn worker_script(app: &AppHandle) -> Result<PathBuf, String> {
    // Try Tauri resource resolution first
    let mut script_path = app
        .path()
        .resolve("scripts/worker.py", tauri::path::BaseDirectory::Resource)
        .unwrap_or_else(|_| {
            app.path()
                .resource_dir()
                .unwrap()
                .join("scripts")
                .join("worker.py")
        });

    if !script_path.exists() {
        // Fallback for dev mode
        let cwd = std::env::current_dir().unwrap_or_default();
        let fallback_root = cwd.join("scripts").join("worker.py");
        let fallback_tauri = cwd.join("..").join("scripts").join("worker.py");
        
        if fallback_root.exists() {
            script_path = fallback_root;
        } else if fallback_tauri.exists() {
            script_path = fallback_tauri;
        } else {
            return Err(format!("worker.py not found. Searched: {:?} and {:?}", fallback_root, fallback_tauri));
        }
    }
    Ok(script_path)
}

/// Spawn the persistent Python worker and wait for the "ready" signal.
/// Emits `model:status` events so the frontend can show progress.
pub async fn spawn_worker(app: AppHandle, state: &WorkerState) -> Result<(), String> {
    // Shut down any existing worker first
    shutdown_worker(state).await;

    let python = python_exe(&app)?;
    let script = worker_script(&app)?;

    app.emit(
        "model:status",
        ModelStatusEvent {
            status: "loading".into(),
            message: Some("Loading model into GPU…".into()),
            device: None,
        },
    )
    .map_err(|e| e.to_string())?;

    // Resolve ffmpeg/ffprobe paths to pass to the worker
    let ffmpeg_path = crate::audio::ffmpeg_exe().unwrap_or_default();
    let ffprobe_path = crate::audio::ffprobe_exe().unwrap_or_default();

    let mut cmd = tokio::process::Command::new(&python);
    cmd.arg(&script)
        .env("PYTHONIOENCODING", "utf-8")
        .env("FFMPEG_PATH", &ffmpeg_path)
        .env("FFPROBE_PATH", &ffprobe_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null()); // suppress Python warnings / logs

    // On Windows, prevent console windows from appearing
    #[cfg(target_os = "windows")]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = cmd.spawn().map_err(|e| {
        let msg = format!("Failed to spawn worker: {}", e);
        let _ = app.emit(
            "model:status",
            ModelStatusEvent {
                status: "error".into(),
                message: Some(msg.clone()),
                device: None,
            },
        );
        msg
    })?;

    let stdin = child.stdin.take().ok_or("Failed to capture worker stdin")?;
    let stdout = child.stdout.take().ok_or("Failed to capture worker stdout")?;

    // Read the first line — should be the ready signal
    let mut reader = BufReader::new(stdout);
    let mut first_line = String::new();

    // Give the model up to 120 seconds to load
    match tokio::time::timeout(
        std::time::Duration::from_secs(120),
        reader.read_line(&mut first_line),
    )
    .await
    {
        Ok(Ok(0)) | Ok(Err(_)) => {
            let _ = child.kill().await;
            let msg = "Worker exited before sending ready signal".to_string();
            let _ = app.emit(
                "model:status",
                ModelStatusEvent {
                    status: "error".into(),
                    message: Some(msg.clone()),
                    device: None,
                },
            );
            return Err(msg);
        }
        Err(_) => {
            let _ = child.kill().await;
            let msg = "Timed out waiting for model to load (120s)".to_string();
            let _ = app.emit(
                "model:status",
                ModelStatusEvent {
                    status: "error".into(),
                    message: Some(msg.clone()),
                    device: None,
                },
            );
            return Err(msg);
        }
        Ok(Ok(_)) => {}
    }

    let response: WorkerResponse =
        serde_json::from_str(first_line.trim()).map_err(|e| format!("Bad ready signal: {}", e))?;

    if response.status == "error" {
        let _ = child.kill().await;
        let msg = response
            .message
            .unwrap_or_else(|| "Unknown worker error".into());
        let _ = app.emit(
            "model:status",
            ModelStatusEvent {
                status: "error".into(),
                message: Some(msg.clone()),
                device: None,
            },
        );
        return Err(msg);
    }

    let device = response.device.clone().unwrap_or_else(|| "unknown".into());

    // Store the worker handle
    {
        let mut guard = state.inner.lock().await;
        *guard = Some(WorkerInner { stdin, child });
    }

    // Spawn a background task to read stdout and emit job events
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        stdout_reader_loop(app_clone, reader).await;
    });

    app.emit(
        "model:status",
        ModelStatusEvent {
            status: "ready".into(),
            message: None,
            device: Some(device),
        },
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Background loop that reads JSON lines from worker stdout and emits
/// Tauri events for each job result.
async fn stdout_reader_loop(app: AppHandle, mut reader: BufReader<tokio::process::ChildStdout>) {
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) | Err(_) => {
                // Worker process exited
                let _ = app.emit(
                    "model:status",
                    ModelStatusEvent {
                        status: "error".into(),
                        message: Some("Worker process exited unexpectedly".into()),
                        device: None,
                    },
                );
                break;
            }
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if let Ok(resp) = serde_json::from_str::<WorkerResponse>(trimmed) {
                    if let Some(id) = &resp.id {
                        match resp.status.as_str() {
                            "done" => {
                                let _ = app.emit(
                                    "job:done",
                                    JobEvent {
                                        id,
                                        message: None,
                                    },
                                );
                            }
                            "error" => {
                                let _ = app.emit(
                                    "job:error",
                                    JobEvent {
                                        id,
                                        message: resp.message.clone(),
                                    },
                                );
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}

/// Send a single enhance job to the worker via stdin.
pub async fn send_job(state: &WorkerState, cmd: &EnhanceCommand) -> Result<(), String> {
    let mut guard = state.inner.lock().await;
    let inner = guard
        .as_mut()
        .ok_or("Worker is not running. Please load the model first.")?;

    let json = serde_json::to_string(cmd).map_err(|e| e.to_string())?;
    inner
        .stdin
        .write_all(format!("{}\n", json).as_bytes())
        .await
        .map_err(|e| format!("Failed to send command to worker: {}", e))?;
    inner
        .stdin
        .flush()
        .await
        .map_err(|e| format!("Failed to flush worker stdin: {}", e))?;

    Ok(())
}

/// Gracefully shut down the worker process.
pub async fn shutdown_worker(state: &WorkerState) {
    let mut guard = state.inner.lock().await;
    if let Some(mut inner) = guard.take() {
        // Try graceful shutdown
        let shutdown = serde_json::json!({"cmd": "shutdown"});
        let _ = inner
            .stdin
            .write_all(format!("{}\n", shutdown).as_bytes())
            .await;
        let _ = inner.stdin.flush().await;

        // Give it a moment, then force kill
        match tokio::time::timeout(std::time::Duration::from_secs(5), inner.child.wait()).await {
            Ok(_) => {}
            Err(_) => {
                let _ = inner.child.kill().await;
            }
        }
    }
}

/// Check if the worker is currently alive and ready.
pub async fn is_ready(state: &WorkerState) -> bool {
    let guard = state.inner.lock().await;
    guard.is_some()
}
