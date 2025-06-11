// src-tauri/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::{AppHandle, Manager, State};
use tokio::time::{Duration};
use tauri::Emitter;
use std::process::Command;
use reqwest;
use futures::StreamExt;
use url::Url;
use chrono::{DateTime, Local};
use tauri_plugin_notification::NotificationExt;
use tokio::io::AsyncWriteExt;


// ... (keep all the structs and enums as they were)

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
enum DownloadStatus {
    Queued,
    Downloading,
    Paused,
    Completed,
    Failed,
    Verifying,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct DownloadTask {
    id: String,
    url: String,
    status: DownloadStatus,
    progress: f64,
    file_name: String,
    save_path: String,
    total_size: u64,
    downloaded_size: u64,
    speed: u64,
    time_remaining: Option<u64>,
    resume_capability: bool,
    error_message: Option<String>,
    created_at: DateTime<Local>,
    completed_at: Option<DateTime<Local>>,
    file_type: String,
    connections: u8,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct AppSettings {
    download_folder: String,
    max_concurrent_downloads: u32,
    max_connections_per_download: u8,
    auto_start: bool,
    show_notifications: bool,
    min_split_size: u64,
}

impl Default for AppSettings {
    fn default() -> Self {
        let download_folder = dirs::download_dir()
            .unwrap_or_else(|| PathBuf::from("~/Downloads"))
            .to_string_lossy()
            .to_string();
            
        Self {
            download_folder,
            max_concurrent_downloads: 4,
            max_connections_per_download: 8,
            auto_start: true,
            show_notifications: true,
            min_split_size: 10 * 1024 * 1024, // 10MB
        }
    }
}

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

struct AppState {
    persistent: Arc<Mutex<PersistentState>>,
    download_handles: Arc<Mutex<std::collections::HashMap<String, tokio::task::JoinHandle<()>>>>,
}

// Helper functions
fn get_state_path(app_handle: &AppHandle) -> anyhow::Result<PathBuf> {
    let path = app_handle.path().app_data_dir()?.join("state.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(path)
}

async fn save_state(state: &State<'_, AppState>, app_handle: &AppHandle) -> anyhow::Result<()> {
    let path = get_state_path(app_handle)?;
    let state_guard = state.persistent.lock().await;
    fs::write(path, serde_json::to_string_pretty(&*state_guard)?)?;
    Ok(())
}

async fn validate_url(url: &str) -> Result<(String, Option<u64>, Option<String>), String> {
    // First validate URL format
    match Url::parse(url) {
        Ok(parsed_url) => {
            if parsed_url.scheme() != "http" && parsed_url.scheme() != "https" {
                return Err("Only HTTP and HTTPS URLs are supported".to_string());
            }
        }
        Err(_) => return Err("Invalid URL format".to_string()),
    }
    
    // Then make a HEAD request to validate and get file info
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
        
    let response = client.head(url)
        .send()
        .await
        .map_err(|e| format!("Failed to connect: {}", e))?;
        
    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }
    
    let content_length = response.content_length();
    let content_type = response.headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    
    Ok((url.to_string(), content_length, content_type))
}

fn extract_filename_from_url(url: &str, content_type: Option<&str>) -> String {
    if let Ok(parsed_url) = Url::parse(url) {
        if let Some(segments) = parsed_url.path_segments() {
            if let Some(last_segment) = segments.last() {
                if !last_segment.is_empty() && last_segment.contains('.') {
                    return last_segment.to_string();
                }
            }
        }
    }
    
    // Generate filename based on content type if available
    let extension = match content_type {
        Some(ct) if ct.contains("video/mp4") => "mp4",
        Some(ct) if ct.contains("video/") => "video",
        Some(ct) if ct.contains("audio/") => "mp3",
        Some(ct) if ct.contains("image/") => "jpg",
        Some(ct) if ct.contains("application/pdf") => "pdf",
        Some(ct) if ct.contains("application/zip") => "zip",
        _ => "bin",
    };
    
    format!("download_{}.{}", chrono::Local::now().timestamp(), extension)
}

fn get_file_type(filename: &str) -> String {
    let extension = filename.split('.').last().unwrap_or("").to_lowercase();
    match extension.as_str() {
        "mp4" | "avi" | "mkv" | "mov" | "wmv" => "Video",
        "mp3" | "wav" | "flac" | "aac" | "ogg" => "Audio",
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" => "Image",
        "zip" | "rar" | "7z" | "tar" | "gz" => "Archive",
        "exe" | "msi" | "dmg" | "deb" | "rpm" => "Executable",
        "pdf" | "doc" | "docx" | "txt" | "odt" => "Document",
        _ => "Other",
    }.to_string()
}

// Tauri Commands
#[tauri::command]
async fn get_all_downloads(state: State<'_, AppState>) -> Result<Vec<DownloadTask>, String> {
    Ok(state.persistent.lock().await.downloads.clone())
}

#[tauri::command]
async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    Ok(state.persistent.lock().await.settings.clone())
}

#[tauri::command]
async fn update_settings(
    settings: AppSettings,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    state.persistent.lock().await.settings = settings;
    save_state(&state, &app_handle).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn choose_download_folder(app_handle: AppHandle) -> Result<String, String> {
    use tauri::api::dialog::blocking::FileDialogBuilder;
    
    let folder = FileDialogBuilder::new()
        .set_title("Choose Download Folder")
        .pick_folder();
        
    match folder {
        Some(path) => Ok(path.to_string_lossy().to_string()),
        None => Err("No folder selected".to_string()),
    }
}

#[tauri::command]
async fn add_download(
    url: String,
    custom_path: Option<String>,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<DownloadTask, String> {
    // Validate URL and get file info
    let (validated_url, content_length, content_type) = validate_url(&url).await?;
    
    let id = format!("task-{}", uuid::Uuid::new_v4());
    let file_name = extract_filename_from_url(&validated_url, content_type.as_deref());
    let file_type = get_file_type(&file_name);
    
    let save_path = custom_path.unwrap_or_else(|| {
        state.persistent.blocking_lock().settings.download_folder.clone()
    });

    let new_task = DownloadTask {
        id: id.clone(),
        url: validated_url.clone(),
        status: DownloadStatus::Queued,
        progress: 0.0,
        file_name,
        save_path,
        total_size: content_length.unwrap_or(0),
        downloaded_size: 0,
        speed: 0,
        time_remaining: None,
        resume_capability: false,
        error_message: None,
        created_at: Local::now(),
        completed_at: None,
        file_type,
        connections: state.persistent.blocking_lock().settings.max_connections_per_download,
    };

    // Add to state
    state.persistent.lock().await.downloads.push(new_task.clone());
    save_state(&state, &app_handle).await.map_err(|e| e.to_string())?;
    
    // Emit update
    app_handle.emit("task_updated", &new_task).unwrap();
    
    // Start download if auto-start is enabled
    if state.persistent.lock().await.settings.auto_start {
        start_download_task(id, state.clone(), app_handle.clone()).await?;
    }
    
    Ok(new_task)
}


#[tauri::command]
async fn pause_download(
    id: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    if let Some(handle) = state.download_handles.lock().await.remove(&id) {
        handle.abort();
    }
    
    let mut state_guard = state.persistent.lock().await;
    if let Some(task) = state_guard.downloads.iter_mut().find(|t| t.id == id) {
        task.status = DownloadStatus::Paused;
        task.speed = 0;
        app_handle.emit("task_updated", &*task).unwrap();
    }
    drop(state_guard);
    
    save_state(&state, &app_handle).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn resume_download(
    id: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    start_download_task(id, state, app_handle).await
}

#[tauri::command]
async fn cancel_download(
    id: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    if let Some(handle) = state.download_handles.lock().await.remove(&id) {
        handle.abort();
    }
    
    state.persistent.lock().await.downloads.retain(|t| t.id != id);
    
    save_state(&state, &app_handle).await.map_err(|e| e.to_string())?;
    
    app_handle.emit("download_removed", &id).unwrap();
    Ok(())
}

#[tauri::command]
async fn open_file(path: String) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);
    
    if !path_buf.exists() {
        return Err("File not found".to_string());
    }
    
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

#[tauri::command]
async fn open_folder(path: String) -> Result<(), String> {
    let folder_path = PathBuf::from(&path);
    
    if !folder_path.exists() {
        return Err("Folder not found".to_string());
    }
    
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

async fn start_download_task(
    id: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let task_info = {
        let mut state_guard = state.persistent.lock().await;
        let task = state_guard.downloads.iter_mut()
            .find(|t| t.id == id)
            .ok_or("Task not found")?;
        
        task.status = DownloadStatus::Downloading;
        app_handle.emit("task_updated", &*task).unwrap();
        
        (task.url.clone(), task.save_path.clone(), task.file_name.clone(), task.downloaded_size)
    };
    
    save_state(&state, &app_handle).await.map_err(|e| e.to_string())?;
    
    let state_clone = state.clone();
    let app_handle_clone = app_handle.clone();
    let id_clone = id.clone();
    
    let handle = tokio::spawn(async move {
        if let Err(e) = download_file(
            id_clone.clone(),
            task_info.0,
            task_info.1,
            task_info.2,
            task_info.3,
            state_clone.clone(),
            app_handle_clone.clone(),
        ).await {
            let mut state_guard = state_clone.persistent.lock().await;
            if let Some(task) = state_guard.downloads.iter_mut().find(|t| t.id == id_clone) {
                task.status = DownloadStatus::Failed;
                task.error_message = Some(e.to_string());
                app_handle_clone.emit("task_updated", &*task).unwrap();
            }
            let _ = save_state(&state_clone, &app_handle_clone).await;
        }
    });
    
    state.download_handles.lock().await.insert(id, handle);
    Ok(())
}

async fn download_file(
    id: String,
    url: String,
    save_path: String,
    file_name: String,
    resume_from: u64,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> anyhow::Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;
    
    let mut request = client.get(&url);
    if resume_from > 0 {
        request = request.header("Range", format!("bytes={}-", resume_from));
    }
    
    let response = request.send().await?;
    
    let resume_capability = response.headers()
        .get("accept-ranges")
        .map(|v| v == "bytes")
        .unwrap_or(false);
    
    let total_size = if resume_from > 0 && response.status() == 206 {
        response.content_length().unwrap_or(0) + resume_from
    } else {
        response.content_length().unwrap_or(0)
    };
    
    {
        let mut state_guard = state.persistent.lock().await;
        if let Some(task) = state_guard.downloads.iter_mut().find(|t| t.id == id) {
            task.total_size = total_size;
            task.resume_capability = resume_capability;
            app_handle.emit("task_updated", &*task).unwrap();
        }
    }
    
    let file_path = PathBuf::from(&save_path).join(&file_name);
    
    if let Some(parent) = file_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    
    let mut file = if resume_from > 0 {
        tokio::fs::OpenOptions::new()
            .append(true)
            .open(&file_path)
            .await?
    } else {
        tokio::fs::File::create(&file_path).await?
    };
    
    let mut stream = response.bytes_stream();
    let mut downloaded = resume_from;
    let mut last_update = std::time::Instant::now();
    let mut last_downloaded = downloaded;
    
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        
        if last_update.elapsed() > Duration::from_millis(100) {
            let speed = ((downloaded - last_downloaded) as f64 / last_update.elapsed().as_secs_f64()) as u64;
            let time_remaining = if speed > 0 {
                Some((total_size - downloaded) / speed)
            } else {
                None
            };
            
            let progress = if total_size > 0 {
                (downloaded as f64 / total_size as f64) * 100.0
            } else {
                0.0
            };
            
            {
                let mut state_guard = state.persistent.lock().await;
                if let Some(task) = state_guard.downloads.iter_mut().find(|t| t.id == id) {
                    task.downloaded_size = downloaded;
                    task.progress = progress;
                    task.speed = speed;
                    task.time_remaining = time_remaining;
                    app_handle.emit("task_updated", &*task).unwrap();
                }
            }
            
            last_update = std::time::Instant::now();
            last_downloaded = downloaded;
        }
    }
    
    {
        let mut state_guard = state.persistent.lock().await;
        if let Some(task) = state_guard.downloads.iter_mut().find(|t| t.id == id) {
            task.status = DownloadStatus::Verifying;
            app_handle.emit("task_updated", &*task).unwrap();
        }
    }
    
    let metadata = tokio::fs::metadata(&file_path).await?;
    if total_size > 0 && metadata.len() != total_size {
        return Err(anyhow::anyhow!("File size mismatch: expected {}, got {}", total_size, metadata.len()));
    }
    
    {
        let mut state_guard = state.persistent.lock().await;
        if let Some(task) = state_guard.downloads.iter_mut().find(|t| t.id == id) {
            task.status = DownloadStatus::Completed;
            task.progress = 100.0;
            task.downloaded_size = total_size;
            task.speed = 0;
            task.completed_at = Some(Local::now());
            app_handle.emit("task_updated", &*task).unwrap();
            
            if state_guard.settings.show_notifications {
                let _ = app_handle.notification()
                    .builder()
                    .title("Download Complete")
                    .body(&format!("{} has finished downloading", task.file_name))
                    .show();
            }
        }
    }
    
    let _ = save_state(&state, &app_handle).await;
    Ok(())
}

#[tauri::command]
async fn handle_cli_args(args: Vec<String>) -> Result<(), String> {
    for arg in args.iter().skip(1) {
        if arg.starts_with("http://") || arg.starts_with("https://") {
            return Ok(());
        }
    }
    Ok(())
}

fn main() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_handle = app.handle().clone();
            let state_path = get_state_path(&app_handle)?;
            
            let initial_state = if state_path.exists() {
                let content = fs::read_to_string(state_path)?;
                serde_json::from_str(&content).unwrap_or_default()
            } else {
                PersistentState::default()
            };

            app.manage(AppState {
                persistent: Arc::new(Mutex::new(initial_state)),
                download_handles: Arc::new(Mutex::new(std::collections::HashMap::new())),
            });
            
            let args: Vec<String> = std::env::args().collect();
            for arg in args.iter().skip(1) {
                if arg.starts_with("http://") || arg.starts_with("https://") {
                    app.emit("cli-url", arg).unwrap();
                }
            }
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            add_download,
            get_all_downloads,
            get_settings,
            update_settings,
            pause_download,
            resume_download,
            cancel_download,
            open_file,
            open_folder,
            choose_download_folder,
            handle_cli_args,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
