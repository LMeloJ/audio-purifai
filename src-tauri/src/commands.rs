use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use tauri::{AppHandle, State};

use crate::{audio, queue, QueueState};
use std::path::PathBuf;
use tauri::Manager;

pub fn get_venv_bin_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let app_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let venv_dir = app_dir.join(".venv");
    if cfg!(target_os = "windows") {
        Ok(venv_dir.join("Scripts"))
    } else {
        Ok(venv_dir.join("bin"))
    }
}

#[tauri::command]
pub fn check_environment(app: AppHandle) -> Result<bool, String> {
    let bin_dir = get_venv_bin_dir(&app)?;
    let exe_name = if cfg!(target_os = "windows") { "deepFilter.exe" } else { "deepFilter" };
    Ok(bin_dir.join(exe_name).exists())
}

#[tauri::command]
pub async fn initialize_environment(app: AppHandle) -> Result<(), String> {
    let app_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&app_dir).map_err(|e| e.to_string())?;

    let script_name = if cfg!(target_os = "windows") { "setup-env.ps1" } else { "setup-env.sh" };
    
    // In Tauri v2, app.path().resolve with BaseDirectory::Resource is the robust way to find resources.
    let mut script_path = app.path().resolve(
        format!("scripts/{}", script_name), 
        tauri::path::BaseDirectory::Resource
    ).unwrap_or_else(|_| {
        // Fallback for some dev environments if resolve fails
        app.path().resource_dir().unwrap().join("scripts").join(script_name)
    });

    if !script_path.exists() {
        // Fallback for dev mode where resources might not be copied to target/debug
        let current_dir = std::env::current_dir().unwrap_or_default();
        let fallback = current_dir.join("..").join("scripts").join(script_name);
        if fallback.exists() {
            script_path = fallback;
        }
    }

    let output = if cfg!(target_os = "windows") {
        tokio::process::Command::new("powershell")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-File")
            .arg(&script_path)
            .current_dir(&app_dir)
            .output()
            .await
            .map_err(|e| e.to_string())?
    } else {
        tokio::process::Command::new("bash")
            .arg(&script_path)
            .current_dir(&app_dir)
            .output()
            .await
            .map_err(|e| e.to_string())?
    };

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    Ok(())
}

#[tauri::command]
pub fn probe_wav(path: String) -> Result<audio::WavInfo, String> {
    audio::probe_wav(&path)
}

#[tauri::command]
pub async fn start_queue(
    app: AppHandle,
    state: State<'_, QueueState>,
    jobs: Vec<queue::QueueJob>,
    concurrency: usize,
) -> Result<(), String> {
    let cancel_flag = Arc::new(AtomicBool::new(false));
    {
        let mut guard = state.cancel_flag.lock().await;
        *guard = Some(cancel_flag.clone());
    }
    tauri::async_runtime::spawn(async move {
        let _ = queue::process_queue(app, jobs, concurrency, cancel_flag).await;
    });
    Ok(())
}

#[tauri::command]
pub async fn cancel_queue(state: State<'_, QueueState>) -> Result<(), String> {
    let guard = state.cancel_flag.lock().await;
    if let Some(flag) = guard.as_ref() {
        flag.store(true, Ordering::Relaxed);
    }
    Ok(())
}
