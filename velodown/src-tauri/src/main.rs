// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Serialize, Deserialize};
use std::sync::Mutex;
use tauri::{State, Window};
use tokio::time::{sleep, Duration};
use tauri::Emitter;

// --- 1. The data structures for our application ---

#[derive(Debug, Serialize, Deserialize, Clone)]
enum DownloadStatus {
    Downloading,
    Completed,
    Error,
}

// This is what a single download task looks like.
#[derive(Debug, Serialize, Deserialize, Clone)]
struct DownloadTask {
    id: String,
    url: String,
    status: DownloadStatus,
    progress: u8,
}

// The settings for our app.
#[derive(Debug, Serialize, Deserialize, Clone)]
struct AppSettings {
    download_folder: String,
    max_concurrent_downloads: u32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            download_folder: "/Users/you/Downloads".to_string(),
            max_concurrent_downloads: 4,
        }
    }
}

// This struct holds the entire state of our application.
// The Mutex is a "lock" that ensures we don't change the data
// from multiple places at the same time, which is important for safety.
struct AppState {
    downloads: Mutex<Vec<DownloadTask>>,
    settings: Mutex<AppSettings>,
}

// --- 2. The functions our frontend can call ("Commands") ---

#[tauri::command]
fn get_settings(state: State<AppState>) -> Result<AppSettings, String> {
    Ok(state.settings.lock().unwrap().clone())
}

#[tauri::command]
fn update_settings(settings: AppSettings, state: State<AppState>) -> Result<(), String> {
    let mut app_settings = state.settings.lock().unwrap();
    *app_settings = settings;
    Ok(())
}

#[tauri::command]
async fn add_download(url: String, state: State<'_, AppState>, window: Window) -> Result<(), String> {
    let id = format!("task-{}", rand::random::<u16>()); // Simple unique ID

    // Create a new task
    let new_task = DownloadTask {
        id: id.clone(),
        url,
        status: DownloadStatus::Downloading,
        progress: 0,
    };

    // Add it to our shared list of downloads
    state.downloads.lock().unwrap().push(new_task.clone());

    // Tell the frontend a new task has been added/status changed
    window.emit("download_status_changed", &new_task).unwrap();

    // --- SIMULATE THE DOWNLOAD ---
    // In a real app, this is where you'd do the complex segmented downloading.
    // Here, we'll just pretend it's downloading over 10 seconds.
    tokio::spawn(async move {
        for i in 1..=100 {
            sleep(Duration::from_millis(100)).await; // Wait 100ms
            
            // Emit progress update to the frontend
            window.emit("download_progress", serde_json::json!({
                "id": id,
                "progress": i
            })).unwrap();
        }

        // Create the final completed state
        let final_task = DownloadTask {
            id,
            url: new_task.url,
            status: DownloadStatus::Completed,
            progress: 100,
        };
        // Tell the frontend the status has changed to "Completed"
        window.emit("download_status_changed", &final_task).unwrap();
    });

    Ok(())
}


// --- 3. The main function that starts our app ---

fn main() {
    // This is the state we'll share across our application
    let app_state = AppState {
        downloads: Mutex::new(Vec::new()),
        settings: Mutex::new(AppSettings::default()),
    };

    tauri::Builder::default()
        // This makes our app_state available to all commands
        .manage(app_state)
        // This registers all our `#[tauri::command]` functions
        .invoke_handler(tauri::generate_handler![
            add_download,
            get_settings,
            update_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
