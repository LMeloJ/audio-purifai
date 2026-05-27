mod audio;
mod commands;
mod queue;
mod worker;

use std::sync::{atomic::AtomicBool, Arc};

use tokio::sync::Mutex;

/// Combined application state holding the cancel flag and the worker handle.
pub struct AppState {
    pub cancel_flag: Mutex<Option<Arc<AtomicBool>>>,
    pub worker: worker::WorkerState,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            cancel_flag: Mutex::new(None),
            worker: worker::WorkerState::default(),
        }
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::check_environment,
            commands::initialize_environment,
            commands::load_model,
            commands::get_model_status,
            commands::probe_wav,
            commands::start_queue,
            commands::cancel_queue
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
