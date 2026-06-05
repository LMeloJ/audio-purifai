use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use tauri::{AppHandle, Emitter, Manager, State};

use crate::{queue, worker, AppState};
use std::path::PathBuf;

pub fn get_venv_bin_dir(_app: &AppHandle) -> Result<PathBuf, String> {
    let python_name = if cfg!(target_os = "windows") {
        "python.exe"
    } else {
        "python"
    };
    let bin_subdir = if cfg!(target_os = "windows") {
        "Scripts"
    } else {
        "bin"
    };

    // Collect candidate .venv locations
    let mut candidates: Vec<PathBuf> = Vec::new();

    // 1. Next to the current executable (production)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join(".venv"));
        }
    }

    // 2-3. Current working directory and its parent (dev mode)
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join(".venv"));
        candidates.push(cwd.join("..").join(".venv"));
    }

    // Return the first candidate that actually contains a Python executable
    for venv in &candidates {
        let bin_dir = venv.join(bin_subdir);
        if bin_dir.join(python_name).exists() {
            return Ok(bin_dir);
        }
    }

    // Nothing found — return the first candidate so the error message is useful
    let fallback = candidates
        .into_iter()
        .next()
        .unwrap_or_default()
        .join(bin_subdir);
    Ok(fallback)
}

#[tauri::command]
pub fn check_environment(app: AppHandle) -> Result<bool, String> {
    let bin_dir = get_venv_bin_dir(&app)?;
    let exe_name = if cfg!(target_os = "windows") {
        "python.exe"
    } else {
        "python"
    };
    Ok(bin_dir.join(exe_name).exists())
}

#[tauri::command]
pub async fn initialize_environment(app: AppHandle) -> Result<(), String> {
    let app_dir = std::env::current_exe()
        .map(|exe| exe.parent().unwrap().to_path_buf())
        .map_err(|e| e.to_string())?;

    let script_name = if cfg!(target_os = "windows") {
        "setup-env.ps1"
    } else {
        "setup-env.sh"
    };

    let mut script_path = app
        .path()
        .resolve(
            format!("scripts/{}", script_name),
            tauri::path::BaseDirectory::Resource,
        )
        .unwrap_or_else(|_| {
            app.path()
                .resource_dir()
                .unwrap()
                .join("scripts")
                .join(script_name)
        });

    if !script_path.exists() {
        let current_dir = std::env::current_dir().unwrap_or_default();
        let fallback_root = current_dir.join("scripts").join(script_name);
        let fallback_tauri = current_dir.join("..").join("scripts").join(script_name);
        
        if fallback_root.exists() {
            script_path = fallback_root;
        } else if fallback_tauri.exists() {
            script_path = fallback_tauri;
        } else {
            return Err(format!("{} not found. Searched: {:?} and {:?}", script_name, fallback_root, fallback_tauri));
        }
    }

    // PowerShell's -File parameter chokes on Windows extended-length paths (\\?\)
    let mut script_path_str = script_path.to_string_lossy().to_string();
    if script_path_str.starts_with("\\\\?\\") {
        script_path_str = script_path_str[4..].to_string();
    }

    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = tokio::process::Command::new("powershell");
        c.arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-File")
            .arg(&script_path_str)
            .current_dir(&app_dir);
        c
    } else {
        let mut c = tokio::process::Command::new("bash");
        c.arg(&script_path_str).current_dir(&app_dir);
        c
    };

    // Hide console window on Windows
    #[cfg(target_os = "windows")]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| e.to_string())?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let app_clone = app.clone();
    let stdout_task = tokio::spawn(async move {
        use tokio::io::AsyncBufReadExt;
        let mut reader = tokio::io::BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let _ = app_clone.emit("install:log", line);
        }
    });

    let app_clone = app.clone();
    let stderr_task = tokio::spawn(async move {
        use tokio::io::AsyncBufReadExt;
        let mut reader = tokio::io::BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let _ = app_clone.emit("install:log", line);
        }
    });

    let status = child.wait().await.map_err(|e| e.to_string())?;
    let _ = stdout_task.await;
    let _ = stderr_task.await;

    if !status.success() {
        return Err("Installation script failed".into());
    }

    Ok(())
}

#[tauri::command]
pub async fn load_model(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    worker::spawn_worker(app, &state.worker).await
}

#[tauri::command]
pub async fn get_model_status(state: State<'_, AppState>) -> Result<String, String> {
    if worker::is_ready(&state.worker).await {
        Ok("ready".into())
    } else {
        Ok("not_loaded".into())
    }
}

#[tauri::command]
pub fn probe_wav(path: String) -> Result<crate::audio::MediaInfo, String> {
    crate::audio::probe_wav(&path)
}

#[tauri::command]
pub fn probe_media(path: String) -> Result<crate::audio::MediaInfo, String> {
    crate::audio::probe_media(&path)
}

#[tauri::command]
pub fn check_ffmpeg() -> Result<bool, String> {
    Ok(crate::audio::ffmpeg_exe().is_ok() && crate::audio::ffprobe_exe().is_ok())
}

#[tauri::command]
pub async fn start_queue(
    app: AppHandle,
    state: State<'_, AppState>,
    jobs: Vec<queue::QueueJob>,
) -> Result<(), String> {
    let cancel_flag = Arc::new(AtomicBool::new(false));
    {
        let mut guard = state.cancel_flag.lock().await;
        *guard = Some(cancel_flag.clone());
    }
    let worker_state = &state.worker;

    // Verify worker is ready before starting
    if !worker::is_ready(worker_state).await {
        return Err("Model is not loaded. Please wait for it to finish loading.".into());
    }

    // We need to clone what we need for the spawned task
    let app_clone = app.clone();
    let cancel_clone = cancel_flag.clone();

    // We can't move worker_state into the spawn, so we process inline
    // but wrap it in a spawn to avoid blocking the command
    let _ = queue::process_queue(app_clone, worker_state, jobs, cancel_clone).await;

    Ok(())
}

#[tauri::command]
pub async fn cancel_queue(state: State<'_, AppState>) -> Result<(), String> {
    let guard = state.cancel_flag.lock().await;
    if let Some(flag) = guard.as_ref() {
        flag.store(true, Ordering::Relaxed);
    }
    Ok(())
}
