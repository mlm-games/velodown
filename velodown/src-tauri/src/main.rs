// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State, Window};
use tokio::time::{sleep, Duration};
use tauri::Emitter;

// --- 1. Enhanced Data Structures ---

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
enum DownloadStatus {
    Queued,
    Downloading,
    Paused,
    Completed,
    Error,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct DownloadTask {
    id: String,
    url: String,
    status: DownloadStatus,
    progress: u8,
    file_name: String,
    save_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct AppSettings {
    download_folder: String,
    max_concurrent_downloads: u32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            // Use a placeholder that can be changed
            download_folder: "~/Downloads".to_string(),
            max_concurrent_downloads: 4,
        }
    }
}

// --- 2. Centralized, Persistent State ---

#[derive(Debug, Serialize, Deserialize)]
struct PersistentState {
    downloads: Vec<DownloadTask>,
    settings: AppSettings,
}

impl Default for PersistentState {
    fn default() -> Self {
        Self {
            downloads: Vec::new(),
            settings: AppSettings::default(),
        }
    }
}

struct AppState(Mutex<PersistentState>); 

// --- 3. Helper Functions for Persistence ---

// Gets the path to our state file (e.g., /home/user/.config/velodown/state.json)
fn get_state_path(app_handle: &AppHandle) -> anyhow::Result<PathBuf> {
    let path = app_handle.path().app_data_dir()?.join("state.json");
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(path)
}

// Saves the entire application state to the JSON file.
fn save_state(state: &State<AppState>, app_handle: &AppHandle) -> anyhow::Result<()> {
    let path = get_state_path(app_handle)?;
    let state_guard = state.0.lock().unwrap();
    fs::write(path, serde_json::to_string_pretty(&*state_guard)?)?;
    log::info!("Application state saved.");
    Ok(())
}

// --- 4. Refactored Tauri Commands ---

#[tauri::command]
fn get_all_downloads(state: State<AppState>) -> Result<Vec<DownloadTask>, String> {
    Ok(state.0.lock().unwrap().downloads.clone())
}

#[tauri::command]
fn get_settings(state: State<AppState>) -> Result<AppSettings, String> {
    Ok(state.0.lock().unwrap().settings.clone())
}

#[tauri::command]
fn update_settings(
    settings: AppSettings,
    state: State<AppState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    state.0.lock().unwrap().settings = settings;
    save_state(&state, &app_handle).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn add_download(
    url: String,
    state: State<'_, AppState>,
    window: Window, // We still use this for the initial emit
    app_handle: AppHandle,
) -> Result<(), String> {
    log::info!("Received add_download command for URL: {}", url);

    let id = format!("task-{}", rand::random::<u32>());
    let file_name = url.split('/').last().unwrap_or("downloaded_file").to_string();
    let save_path = state.0.lock().unwrap().settings.download_folder.clone();

    let new_task = DownloadTask {
        id: id.clone(),
        url,
        status: DownloadStatus::Queued,
        progress: 0,
        file_name,
        save_path,
    };

    state.0.lock().unwrap().downloads.push(new_task.clone());
    save_state(&state, &app_handle).map_err(|e| e.to_string())?;
    window.emit("task_updated", &new_task).unwrap();
    log::info!("Task {} queued and UI notified.", id);


    let app_handle_clone = app_handle.clone();
    let id_clone = id.clone();


    // SIMULATE THE DOWNLOAD IN THE BACKGROUND
    tokio::spawn(async move {
        // Inside the background thread, get new handles to state and the window
        // using the 'static AppHandle. This is the key to solving the lifetime error.
        let state = app_handle_clone.state::<AppState>();
        let window = app_handle_clone.get_webview_window("main").expect("Failed to get main window");

        // Now, the rest of the logic works perfectly with these new, valid handles.

        // Change status to Downloading
        {
            let mut state_guard = state.0.lock().unwrap();
            if let Some(task) = state_guard.downloads.iter_mut().find(|t| t.id == id_clone) {
                task.status = DownloadStatus::Downloading;
                window.emit("task_updated", &*task).unwrap();
                log::info!("Task {} status changed to Downloading.", id_clone);
            }
        }
        save_state(&state, &app_handle_clone).unwrap_or_else(|e| log::error!("Failed to save state: {}", e));

        // Simulate progress
        for i in 1..=100 {
            sleep(Duration::from_millis(50)).await;
            window.emit("download_progress", serde_json::json!({ "id": id_clone, "progress": i })).unwrap();
        }

        // Finalize and mark as completed
        {
            let mut state_guard = state.0.lock().unwrap();
            if let Some(task) = state_guard.downloads.iter_mut().find(|t| t.id == id_clone) {
                task.status = DownloadStatus::Completed;
                task.progress = 100;
                window.emit("task_updated", &*task).unwrap();
                log::info!("Task {} status changed to Completed.", id_clone);
            }
        }
        save_state(&state, &app_handle_clone).unwrap_or_else(|e| log::error!("Failed to save state: {}", e));
    });

    Ok(())
}




// --- 5. Main Function with Setup Hook for Loading State ---

fn main() {
    env_logger::init(); // Initialize the logger

    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle().clone();
            let state_path = get_state_path(&app_handle)
                .expect("Failed to get state path");
            
            let initial_state = if state_path.exists() {
                log::info!("Loading state from {}", state_path.display());
                let content = fs::read_to_string(state_path)
                    .expect("Failed to read state file");
                serde_json::from_str(&content)
                    .unwrap_or_else(|e| {
                        log::error!("Failed to parse state file, using default: {}", e);
                        PersistentState::default()
                    })
            } else {
                log::info!("No state file found, using default state.");
                PersistentState::default()
            };

            app.manage(AppState(Mutex::new(initial_state)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            add_download,
            get_all_downloads,
            get_settings,
            update_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
