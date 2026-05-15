use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use tauri::{AppHandle, State};

use crate::{audio, queue, QueueState};

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
