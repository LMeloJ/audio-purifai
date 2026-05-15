mod audio;
mod commands;
mod queue;

use std::sync::{atomic::AtomicBool, Arc};

use tokio::sync::Mutex;

#[derive(Default)]
pub struct QueueState {
    pub cancel_flag: Mutex<Option<Arc<AtomicBool>>>,
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(QueueState::default())
        .invoke_handler(tauri::generate_handler![
            commands::probe_wav,
            commands::start_queue,
            commands::cancel_queue
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
