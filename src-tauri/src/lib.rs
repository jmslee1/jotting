// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use notify::{RecommendedWatcher, RecursiveMode, Watcher, Event};
use tauri::{Emitter, Manager, State};

#[derive(Serialize, Deserialize, Clone)]
pub struct NoteMeta {
    pub filename: String,
    pub modified: u64,
}

struct WatcherState(Mutex<Option<RecommendedWatcher>>);

#[tauri::command]
fn list_notes(vault_path: String) -> Result<Vec<NoteMeta>, String> {
    let path = PathBuf::from(&vault_path);
    if !path.exists() {
        return Err(format!("Vault path does not exist: {}", vault_path));
    }

    let mut notes = Vec::new();
    let entries = fs::read_dir(&path).map_err(|e| e.to_string())?;

    for entry in entries.flatten() {
        let p = entry.path();
        if p.extension().and_then(|s| s.to_str()) == Some("md") {
            let filename = p.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            let modified = entry.metadata()
                .and_then(|m| m.modified())
                .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs())
                .unwrap_or(0);
            notes.push(NoteMeta { filename, modified });
        }
    }

    notes.sort_by(|a, b| b.modified.cmp(&a.modified));
    Ok(notes)
}

#[tauri::command]
fn read_note(vault_path: String, filename: String) -> Result<String, String> {
    let path = PathBuf::from(&vault_path).join(&filename);
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn write_note(vault_path: String, filename: String, content: String) -> Result<(), String> {
    let path = PathBuf::from(&vault_path).join(&filename);
    fs::write(&path, content).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_note(vault_path: String) -> Result<String, String> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let filename = format!("untitled-{}.md", timestamp);
    let path = PathBuf::from(&vault_path).join(&filename);
    fs::write(&path, "").map_err(|e| e.to_string())?;
    Ok(filename)
}

#[tauri::command]
fn delete_note(vault_path: String, filename: String) -> Result<(), String> {
    let path = PathBuf::from(&vault_path).join(&filename);
    fs::remove_file(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn start_watching(
    vault_path: String,
    app: tauri::AppHandle,
    state: State<WatcherState>,
) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    *guard = None; // drop any existing watcher

    let app_handle = app.clone();
    let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
        if let Ok(event) = res {
            // Only fire on events that change file contents or existence
            use notify::EventKind::*;
            if matches!(event.kind, Create(_) | Modify(_) | Remove(_)) {
                let _ = app_handle.emit("vault-changed", ());
            }
        }
    }).map_err(|e| e.to_string())?;

    watcher
        .watch(&PathBuf::from(&vault_path), RecursiveMode::NonRecursive)
        .map_err(|e| e.to_string())?;

    *guard = Some(watcher);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(WatcherState(Mutex::new(None)))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            list_notes,
            read_note,
            write_note,
            create_note,
            delete_note,
            start_watching
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}