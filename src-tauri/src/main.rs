#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::{AppHandle, Manager, State};
use tokio::time::{Duration, Instant}; 
use tauri::Emitter;
use std::process::Command;
use reqwest::{cookie::Jar, Client};
use futures::StreamExt;
use url::Url;
use chrono::{DateTime, Local};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_dialog::{DialogExt, FilePath};
use tokio::sync::oneshot;
use tokio::io::AsyncWriteExt;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/108.0.0.0 Safari/537.36";

// --- STRUCTS & ENUMS ---

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
enum DownloadStatus { Queued, Downloading, Paused, Completed, Failed, Verifying, Retrying } // NEW: Added Retrying status

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DownloadTask {
    id: String, url: String, status: DownloadStatus, progress: f64, file_name: String,
    save_path: String, total_size: u64, downloaded_size: u64, speed: u64,
    time_remaining: Option<u64>, resume_capability: bool, error_message: Option<String>,
    created_at: DateTime<Local>, completed_at: Option<DateTime<Local>>,
    file_type: String, connections: u8,
    resume_attempts: u8,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct AppSettings {
    download_folder: String, max_concurrent_downloads: u32, max_connections_per_download: u8,
    auto_start: bool, show_notifications: bool, min_split_size: u64,
    auto_resume_downloads: bool,
    max_resume_attempts: u8,
    resume_delay_seconds: u64,
    min_fail_duration_seconds: u64,
}

impl Default for AppSettings {
    fn default() -> Self {
        let download_folder = dirs::download_dir()
            .unwrap_or_else(|| PathBuf::from("~/Downloads"))
            .to_string_lossy().to_string();
        Self {
            download_folder, max_concurrent_downloads: 4, max_connections_per_download: 8,
            auto_start: true, show_notifications: true, min_split_size: 10 * 1024 * 1024,
            auto_resume_downloads: true,
            max_resume_attempts: 5,
            resume_delay_seconds: 10,
            min_fail_duration_seconds: 20,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct PersistentState { downloads: Vec<DownloadTask>, settings: AppSettings }
impl Default for PersistentState { fn default() -> Self { Self { downloads: Vec::new(), settings: AppSettings::default() } } }

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DownloadInfo {
    final_url: String, file_name: String, total_size: Option<u64>, file_type: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddDownloadPayload {
    url: String, file_name: String, total_size: Option<u64>, custom_path: Option<String>,
}

struct AppState {
    persistent: Arc<Mutex<PersistentState>>,
    download_handles: Arc<Mutex<std::collections::HashMap<String, tokio::task::JoinHandle<()>>>>,
}

// --- HELPER FUNCTIONS (Unchanged) ---
fn get_filename_from_response(response: &reqwest::Response, url: &Url) -> String {
    if let Some(cd) = response.headers().get("content-disposition") {
        if let Ok(cd_str) = cd.to_str() {
            if let Some(filename_part) = cd_str.split(';').find(|s| s.trim().starts_with("filename=")) {
                let filename = filename_part.trim().trim_start_matches("filename=").trim_matches('"');
                if !filename.is_empty() { return filename.to_string(); }
            }
        }
    }
    if let Some(segments) = url.path_segments() {
        if let Some(last_segment) = segments.last() {
            if !last_segment.is_empty() { return last_segment.to_string(); }
        }
    }
    format!("download_{}.tmp", chrono::Local::now().timestamp())
}
fn get_file_type(filename: &str) -> String {
    let extension = filename.split('.').last().unwrap_or("").to_lowercase();
    match extension.as_str() {
        "mp4" | "avi" | "mkv" | "mov" | "wmv" => "Video", "mp3" | "wav" | "flac" | "aac" | "ogg" => "Audio",
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" => "Image", "zip" | "rar" | "7z" | "tar" | "gz" => "Archive",
        "exe" | "msi" | "dmg" | "deb" | "rpm" => "Executable", "pdf" | "doc" | "docx" | "txt" | "odt" => "Document",
        _ => "Other",
    }.to_string()
}
fn get_state_path(app_handle: &AppHandle) -> anyhow::Result<PathBuf> {
    let path = app_handle.path().app_data_dir()?.join("state.json");
    if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
    Ok(path)
}
async fn save_state(state: &State<'_, AppState>, app_handle: &AppHandle) -> anyhow::Result<()> {
    let path = get_state_path(app_handle)?;
    let state_guard = state.persistent.lock().await;
    fs::write(path, serde_json::to_string_pretty(&*state_guard)?)?;
    Ok(())
}

// --- TAURI COMMANDS ---

#[tauri::command]
async fn get_download_info(url: String) -> Result<DownloadInfo, String> {
    let cookie_jar = Arc::new(Jar::default());
    let client = Client::builder()
        .user_agent(USER_AGENT)
        .redirect(reqwest::redirect::Policy::default())
        .cookie_provider(cookie_jar) // Use the cookie jar
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client.get(&url)
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8")
        .header("Accept-Language", "en-US,en;q=0.5")
        .header("Referer", &url) // Add a Referer header
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }

    let final_url = response.url().to_string();
    let file_name = get_filename_from_response(&response, response.url());
    let total_size = response.content_length();
    let file_type = get_file_type(&file_name);

    Ok(DownloadInfo { final_url, file_name, total_size, file_type })
}

#[tauri::command]
async fn choose_download_folder(app_handle: AppHandle) -> Result<String, String> {
    let (tx, rx) = oneshot::channel();
    app_handle.dialog().file().set_title("Choose Download Folder")
        .pick_folder(move |folder_path: Option<FilePath>| {
            let _ = tx.send(folder_path);
        });
    match rx.await {
        Ok(Some(file_path)) => match file_path.as_path() {
            Some(path) => Ok(path.to_string_lossy().to_string()),
            None => Err("Invalid path returned from dialog".to_string()),
        },
        Ok(None) => Err("No folder selected".to_string()),
        Err(_) => Err("Dialog was cancelled".to_string()),
    }
}

// This command now correctly receives the final URL from the info-fetch step
#[tauri::command]
async fn add_download(payload: AddDownloadPayload, state: State<'_, AppState>, app_handle: AppHandle) -> Result<DownloadTask, String> {
    let id = format!("task-{}", uuid::Uuid::new_v4());
    let file_type = get_file_type(&payload.file_name);
    let (default_save_path, max_connections, auto_start) = {
        let settings = &state.persistent.lock().await.settings;
        (settings.download_folder.clone(), settings.max_connections_per_download, settings.auto_start)
    };
    let save_path = payload.custom_path.unwrap_or(default_save_path);
    let new_task = DownloadTask {
        id: id.clone(), url: payload.url, status: DownloadStatus::Queued, progress: 0.0,
        file_name: payload.file_name, save_path, total_size: payload.total_size.unwrap_or(0),
        downloaded_size: 0, speed: 0, time_remaining: None, resume_capability: false,
        error_message: None, created_at: Local::now(), completed_at: None,
        file_type, connections: max_connections,
        resume_attempts: 0, // NEW: Initialize to 0
    };
    state.persistent.lock().await.downloads.push(new_task.clone());
    save_state(&state, &app_handle).await.map_err(|e| e.to_string())?;
    app_handle.emit("task_updated", &new_task).unwrap();
    if auto_start { start_download_task(id, app_handle.clone()).await?; }
    Ok(new_task)
}

#[tauri::command]
async fn get_all_downloads(state: State<'_, AppState>) -> Result<Vec<DownloadTask>, String> { Ok(state.persistent.lock().await.downloads.clone()) }
#[tauri::command]
async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> { Ok(state.persistent.lock().await.settings.clone()) }
#[tauri::command(rename_all = "camelCase")]
async fn update_settings(settings: AppSettings, state: State<'_, AppState>, app_handle: AppHandle) -> Result<(), String> {
    state.persistent.lock().await.settings = settings;
    save_state(&state, &app_handle).await.map_err(|e| e.to_string())?;
    Ok(())
}
#[tauri::command]
async fn pause_download(id: String, state: State<'_, AppState>, app_handle: AppHandle) -> Result<(), String> {
    if let Some(handle) = state.download_handles.lock().await.remove(&id) { handle.abort(); }
    let mut state_guard = state.persistent.lock().await;
    if let Some(task) = state_guard.downloads.iter_mut().find(|t| t.id == id) {
        task.status = DownloadStatus::Paused; task.speed = 0;
        app_handle.emit("task_updated", &*task).unwrap();
    }
    drop(state_guard); save_state(&state, &app_handle).await.map_err(|e| e.to_string())?;
    Ok(())
}
#[tauri::command]
async fn resume_download(id: String, app_handle: AppHandle) -> Result<(), String> { start_download_task(id, app_handle).await }
#[tauri::command]
async fn cancel_download(id: String, state: State<'_, AppState>, app_handle: AppHandle) -> Result<(), String> {
    if let Some(handle) = state.download_handles.lock().await.remove(&id) { handle.abort(); }
    state.persistent.lock().await.downloads.retain(|t| t.id != id);
    save_state(&state, &app_handle).await.map_err(|e| e.to_string())?;
    app_handle.emit("download_removed", &id).unwrap();
    Ok(())
}
#[tauri::command]
async fn open_file(save_path: String, file_name: String) -> Result<(), String> {
    // Let Rust's PathBuf handle joining paths correctly for any OS
    let full_path = PathBuf::from(&save_path).join(&file_name);

    if !full_path.exists() {
        return Err("File not found".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer").arg(&full_path).spawn().map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(&full_path).spawn().map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open").arg(&full_path).spawn().map_err(|e| e.to_string())?;
    }
    
    Ok(())
}
#[tauri::command]
async fn open_folder(path: String) -> Result<(), String> {
    let folder_path = PathBuf::from(&path); if !folder_path.exists() { return Err("Folder not found".to_string()); }
    #[cfg(target_os = "windows")] { Command::new("explorer").arg(&path).spawn().map_err(|e| e.to_string())?; }
    #[cfg(target_os = "macos")] { Command::new("open").arg(&path).spawn().map_err(|e| e.to_string())?; }
    #[cfg(target_os = "linux")] { Command::new("xdg-open").arg(&path).spawn().map_err(|e| e.to_string())?; }
    Ok(())
}
async fn start_download_task(id: String, app_handle: AppHandle) -> Result<(), String> {
    let app_handle_clone = app_handle.clone();
    let id_clone = id.clone();

    let handle = tokio::spawn(async move {
        let settings = {
            let state: State<AppState> = app_handle_clone.state();
            let p_state = state.persistent.lock().await;
            p_state.settings.clone()
        };

        loop {
            let task_info = {
                let state: State<AppState> = app_handle_clone.state();
                let mut p_state = state.persistent.lock().await;
                if let Some(task) = p_state.downloads.iter_mut().find(|t| t.id == id_clone) {
                    // Only increment attempts if it's not the very first run
                    if task.status != DownloadStatus::Queued {
                        task.resume_attempts += 1;
                    }
                    task.status = DownloadStatus::Downloading;
                    app_handle_clone.emit("task_updated", &*task).unwrap();
                    Some((
                        task.url.clone(), task.save_path.clone(), task.file_name.clone(),
                        task.downloaded_size, task.resume_attempts
                    ))
                } else {
                    None
                }
            };

            let (url, save_path, file_name, downloaded_size, attempts) = match task_info {
                Some(info) => info,
                None => break,
            };

            let attempt_start_time = Instant::now();
            
            // Clone the values right before they are moved
            let result = download_file(
                &id_clone, 
                &url,      
                &save_path,
                &file_name,
                downloaded_size,
                &app_handle_clone,
            ).await;

            if result.is_ok() {
                break;
            }

            let attempt_duration = attempt_start_time.elapsed();
            let error_string = result.err().unwrap().to_string();

            // Check for conditions where we should NOT retry
            let should_fail_permanently = 
                !settings.auto_resume_downloads ||
                attempts >= settings.max_resume_attempts ||
                (attempts > 0 && attempt_duration < Duration::from_secs(settings.min_fail_duration_seconds)) || // Added attempts > 0 check
                error_string.contains("403") || error_string.contains("404") || error_string.contains("File size mismatch");

            if should_fail_permanently {
                let state: State<AppState> = app_handle_clone.state();
                let mut p_state = state.persistent.lock().await;
                if let Some(task) = p_state.downloads.iter_mut().find(|t| t.id == id_clone) {
                    task.status = DownloadStatus::Failed;
                    task.error_message = Some(error_string);
                    app_handle_clone.emit("task_updated", &*task).unwrap();
                }
                break;
            } else {
                let state: State<AppState> = app_handle_clone.state();
                let mut p_state = state.persistent.lock().await;
                if let Some(task) = p_state.downloads.iter_mut().find(|t| t.id == id_clone) {
                    task.status = DownloadStatus::Retrying;
                    task.error_message = Some(format!("Network error. Retrying in {}s... (Attempt {})", settings.resume_delay_seconds, attempts));
                    app_handle_clone.emit("task_updated", &*task).unwrap();
                }
                drop(p_state);

                tokio::time::sleep(Duration::from_secs(settings.resume_delay_seconds)).await;
            }
        }

        let state: State<AppState> = app_handle_clone.state();
        state.download_handles.lock().await.remove(&id_clone);
        let _ = save_state(&state, &app_handle_clone).await;
    });
    
    app_handle.state::<AppState>().download_handles.lock().await.insert(id, handle);
    Ok(())
}
async fn download_file(id: &str, url: &str, save_path: &str, file_name: &str, resume_from: u64, app_handle: &AppHandle) -> anyhow::Result<()> {
    let client = Client::builder().user_agent(USER_AGENT).timeout(Duration::from_secs(30)).build()?;
    let mut request = client.get(url); if resume_from > 0 { request = request.header("Range", format!("bytes={}-", resume_from)); }
    let response = request.send().await?; let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Err(anyhow::anyhow!("Authorization failed ({}). The link may be protected or expired.", status));
    }
    if !status.is_success() { return Err(anyhow::anyhow!("Server returned an error: {}", status)); }
    let resume_capability = response.headers().get("accept-ranges").map(|v| v == "bytes").unwrap_or(false);
    let total_size = if resume_from > 0 && response.status() == 206 { response.content_length().unwrap_or(0) + resume_from } else { response.content_length().unwrap_or(0) };
    {
        let state: State<AppState> = app_handle.state(); let mut state_guard = state.persistent.lock().await;
        if let Some(task) = state_guard.downloads.iter_mut().find(|t| t.id == id) {
            task.total_size = total_size; task.resume_capability = resume_capability;
            app_handle.emit("task_updated", &*task).unwrap();
        }
    }
    let file_path = PathBuf::from(&save_path).join(&file_name);
    if let Some(parent) = file_path.parent() { tokio::fs::create_dir_all(parent).await?; }
    let mut file = if resume_from > 0 { tokio::fs::OpenOptions::new().append(true).open(&file_path).await? } else { tokio::fs::File::create(&file_path).await? };
    let mut stream = response.bytes_stream(); let mut downloaded = resume_from;
    let mut last_update = std::time::Instant::now(); let mut last_downloaded = downloaded;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?; file.write_all(&chunk).await?; downloaded += chunk.len() as u64;
        if last_update.elapsed() > Duration::from_millis(100) {
            let speed = ((downloaded - last_downloaded) as f64 / last_update.elapsed().as_secs_f64()) as u64;
            let time_remaining = if speed > 0 { Some((total_size - downloaded) / speed) } else { None };
            let progress = if total_size > 0 { (downloaded as f64 / total_size as f64) * 100.0 } else { 0.0 };
            {
                let state: State<AppState> = app_handle.state(); let mut state_guard = state.persistent.lock().await;
                if let Some(task) = state_guard.downloads.iter_mut().find(|t| t.id == id) {
                    task.downloaded_size = downloaded; task.progress = progress; task.speed = speed; task.time_remaining = time_remaining;
                    app_handle.emit("task_updated", &*task).unwrap();
                }
            }
            last_update = std::time::Instant::now(); last_downloaded = downloaded;
        }
    }
    {
        let state: State<AppState> = app_handle.state(); let mut state_guard = state.persistent.lock().await;
        if let Some(task) = state_guard.downloads.iter_mut().find(|t| t.id == id) {
            task.status = DownloadStatus::Verifying; app_handle.emit("task_updated", &*task).unwrap();
        }
    }
    let metadata = tokio::fs::metadata(&file_path).await?;
    if total_size > 0 && metadata.len() != total_size { return Err(anyhow::anyhow!("File size mismatch")); }
    {
        let state: State<AppState> = app_handle.state(); let mut state_guard = state.persistent.lock().await;
        let show_notifications = state_guard.settings.show_notifications;
        if let Some(task) = state_guard.downloads.iter_mut().find(|t| t.id == id) {
            task.status = DownloadStatus::Completed; task.progress = 100.0; task.downloaded_size = total_size;
            task.speed = 0; task.completed_at = Some(Local::now());
            app_handle.emit("task_updated", &*task).unwrap();
            if show_notifications {
                let _ = app_handle.notification().builder().title("Download Complete").body(&format!("{} has finished downloading", task.file_name)).show();
            }
        }
    }
    let _ = save_state(&app_handle.state(), &app_handle).await;
    Ok(())
}
#[tauri::command]
async fn handle_cli_args(args: Vec<String>) -> Result<(), String> {
    for arg in args.iter().skip(1) { if arg.starts_with("http://") || arg.starts_with("https://") { return Ok(()); } } Ok(())
}
fn main() {
    env_logger::init();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init()).plugin(tauri_plugin_notification::init()).plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_handle = app.handle().clone(); let state_path = get_state_path(&app_handle)?;
            let initial_state = if state_path.exists() {
                let content = fs::read_to_string(state_path)?;
                serde_json::from_str(&content).unwrap_or_default()
            } else { PersistentState::default() };
            app.manage(AppState {
                persistent: Arc::new(Mutex::new(initial_state)),
                download_handles: Arc::new(Mutex::new(std::collections::HashMap::new())),
            });
            let args: Vec<String> = std::env::args().collect();
            for arg in args.iter().skip(1) { if arg.starts_with("http://") || arg.starts_with("https://") { app.emit("cli-url", arg).unwrap(); } }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_download_info, add_download, get_all_downloads, get_settings, update_settings,
            pause_download, resume_download, cancel_download, open_file, open_folder,
            choose_download_folder, handle_cli_args,
        ])
        .run(tauri::generate_context!()).expect("error while running tauri application");
}
