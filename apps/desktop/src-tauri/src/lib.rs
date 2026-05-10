use hash_killer::duplicate_cleaner::{ActivityEvent, CleanReport};
use hash_killer::hash_algorithm::HashAlgorithm;
use hash_killer::scan_mode::ScanMode;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};

#[derive(Clone, Default)]
struct ScanManager {
    active: Arc<Mutex<Option<ActiveScan>>>,
    next_id: Arc<AtomicU64>,
}

#[derive(Clone)]
struct ActiveScan {
    scan_id: u64,
    cancelled: Arc<AtomicBool>,
}

#[derive(Deserialize)]
struct ScanStartRequestDto {
    roots: Vec<String>,
    algorithm: String,
    scan_mode: String,
}

#[derive(Deserialize)]
struct ScanCancelRequestDto {
    scan_id: u64,
}

#[derive(Deserialize)]
struct ValueDto<T> {
    value: T,
}

#[derive(Deserialize)]
struct RootsDto {
    roots: Vec<String>,
}

#[derive(Deserialize)]
struct SaveQuarantineDestinationDto {
    volume_key: String,
    target_path: String,
}

#[derive(Deserialize)]
struct PathDto {
    path: String,
}

#[derive(Deserialize)]
struct PickFolderDto {
    initial_directory: Option<String>,
}

#[derive(Deserialize)]
struct SaveTextFileDto {
    suggested_name: String,
    contents: String,
}

#[derive(Serialize)]
struct ScanRunDto {
    scan_id: u64,
}

#[derive(Serialize)]
struct SettingsDto {
    cache_limit_mb: u64,
    cache_limit_configured: bool,
    algorithm: String,
    algorithm_configured: bool,
    scan_mode: String,
    scan_mode_configured: bool,
}

#[derive(Serialize)]
struct CacheClearDto {
    removed: usize,
}

#[derive(Serialize)]
struct CacheSaveDto {
    pruned: usize,
}

#[derive(Serialize)]
struct CountDto {
    count: usize,
}

#[derive(Serialize)]
struct UnitDto {
    ok: bool,
}

#[derive(Clone, Serialize)]
struct ScanActivityPayload {
    scan_id: u64,
    event: ActivityEvent,
}

#[derive(Clone, Serialize)]
struct ScanCompletedPayload {
    scan_id: u64,
    report: CleanReport,
}

#[derive(Clone, Serialize)]
struct ScanFailedPayload {
    scan_id: u64,
    message: String,
}

#[derive(Clone, Serialize)]
struct ScanCancelledPayload {
    scan_id: u64,
}

pub fn run() {
    tauri::Builder::default()
        .manage(ScanManager::default())
        .invoke_handler(tauri::generate_handler![
            start_scan,
            cancel_scan,
            load_settings,
            save_cache_limit,
            clear_cache,
            save_hash_algorithm,
            save_scan_mode,
            volume_destinations,
            save_quarantine_destination,
            clear_quarantine_destinations,
            reveal_file,
            pick_folders,
            pick_folder,
            save_text_file
        ])
        .run(tauri::generate_context!())
        .expect("failed to run app");
}

#[tauri::command]
fn start_scan(
    app: AppHandle,
    manager: State<'_, ScanManager>,
    request: ScanStartRequestDto,
) -> Result<ScanRunDto, String> {
    let scan_id = manager.next_id.fetch_add(1, Ordering::Relaxed) + 1;
    let cancelled = Arc::new(AtomicBool::new(false));

    {
        let mut active = manager.active.lock().map_err(|error| error.to_string())?;

        if active.is_some() {
            return Err("이미 검사가 실행 중입니다.".to_string());
        }

        *active = Some(ActiveScan {
            scan_id,
            cancelled: cancelled.clone(),
        });
    }

    let manager = manager.inner().clone();
    std::thread::spawn(move || {
        let roots = request
            .roots
            .into_iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        let algorithm = HashAlgorithm::from_id(&request.algorithm);
        let scan_mode = ScanMode::from_id(&request.scan_mode);
        let emit_app = app.clone();
        let emit_cancelled = cancelled.clone();
        let result = hash_killer::duplicate_cleaner::clean_duplicate_paths_with_progress_and_cancel(
            roots,
            algorithm,
            scan_mode,
            move |event| {
                let _ = emit_app.emit("scan://activity", ScanActivityPayload { scan_id, event });
            },
            move || emit_cancelled.load(Ordering::Relaxed),
        );

        manager.clear_scan(scan_id);

        match result {
            Ok(report) => {
                let _ = app.emit("scan://completed", ScanCompletedPayload { scan_id, report });
            }
            Err(_) if cancelled.load(Ordering::Relaxed) => {
                let _ = app.emit("scan://cancelled", ScanCancelledPayload { scan_id });
            }
            Err(message) => {
                let _ = app.emit("scan://failed", ScanFailedPayload { scan_id, message });
            }
        }
    });

    Ok(ScanRunDto { scan_id })
}

#[tauri::command]
fn cancel_scan(
    manager: State<'_, ScanManager>,
    request: ScanCancelRequestDto,
) -> Result<UnitDto, String> {
    let active = manager.active.lock().map_err(|error| error.to_string())?;

    if let Some(scan) = active.as_ref().filter(|scan| scan.scan_id == request.scan_id) {
        scan.cancelled.store(true, Ordering::Relaxed);
    }

    Ok(UnitDto { ok: true })
}

#[tauri::command]
fn load_settings() -> Result<SettingsDto, String> {
    Ok(SettingsDto {
        cache_limit_mb: hash_killer::cache::load_cache_limit_mb()?,
        cache_limit_configured: hash_killer::cache::load_cache_limit_configured()?,
        algorithm: hash_killer::cache::load_hash_algorithm_id()?,
        algorithm_configured: hash_killer::cache::load_hash_algorithm_configured()?,
        scan_mode: hash_killer::cache::load_scan_mode_id()?,
        scan_mode_configured: hash_killer::cache::load_scan_mode_configured()?,
    })
}

#[tauri::command]
fn save_cache_limit(request: ValueDto<u64>) -> Result<CacheSaveDto, String> {
    let pruned = hash_killer::cache::save_cache_limit_mb(request.value)?;
    Ok(CacheSaveDto { pruned })
}

#[tauri::command]
fn clear_cache() -> Result<CacheClearDto, String> {
    let removed = hash_killer::cache::clear_cache()?;
    Ok(CacheClearDto { removed })
}

#[tauri::command]
fn save_hash_algorithm(request: ValueDto<String>) -> Result<UnitDto, String> {
    let algorithm = HashAlgorithm::from_id(&request.value);
    hash_killer::cache::save_hash_algorithm_id(algorithm.id())?;
    Ok(UnitDto { ok: true })
}

#[tauri::command]
fn save_scan_mode(request: ValueDto<String>) -> Result<UnitDto, String> {
    let mode = ScanMode::from_id(&request.value);
    hash_killer::cache::save_scan_mode_id(mode.id())?;
    Ok(UnitDto { ok: true })
}

#[tauri::command]
fn volume_destinations(request: RootsDto) -> Result<Vec<hash_killer::quarantine::VolumeDestination>, String> {
    hash_killer::quarantine::volume_destinations(&request.roots)
}

#[tauri::command]
fn save_quarantine_destination(request: SaveQuarantineDestinationDto) -> Result<UnitDto, String> {
    hash_killer::quarantine::save_destination(
        &request.volume_key,
        &PathBuf::from(request.target_path),
    )?;
    Ok(UnitDto { ok: true })
}

#[tauri::command]
fn clear_quarantine_destinations() -> Result<CountDto, String> {
    let count = hash_killer::quarantine::clear_destinations()?;
    Ok(CountDto { count })
}

#[tauri::command]
fn reveal_file(request: PathDto) -> Result<UnitDto, String> {
    reveal_file_path(PathBuf::from(request.path))?;
    Ok(UnitDto { ok: true })
}

#[tauri::command]
fn pick_folders() -> Result<Vec<String>, String> {
    Ok(rfd::FileDialog::new()
        .pick_folders()
        .unwrap_or_default()
        .into_iter()
        .map(|path| path.display().to_string())
        .collect())
}

#[tauri::command]
fn pick_folder(request: PickFolderDto) -> Result<Option<String>, String> {
    let dialog = request
        .initial_directory
        .map(PathBuf::from)
        .filter(|path| path.exists())
        .map(|path| rfd::FileDialog::new().set_directory(path))
        .unwrap_or_else(rfd::FileDialog::new);

    Ok(dialog
        .pick_folder()
        .map(|path| path.display().to_string()))
}

#[tauri::command]
fn save_text_file(request: SaveTextFileDto) -> Result<UnitDto, String> {
    let Some(path) = rfd::FileDialog::new()
        .set_file_name(&request.suggested_name)
        .save_file()
    else {
        return Ok(UnitDto { ok: false });
    };

    std::fs::write(path, request.contents).map_err(|error| error.to_string())?;
    Ok(UnitDto { ok: true })
}

impl ScanManager {
    fn clear_scan(&self, scan_id: u64) {
        if let Ok(mut active) = self.active.lock() {
            if active.as_ref().is_some_and(|scan| scan.scan_id == scan_id) {
                *active = None;
            }
        }
    }
}

fn reveal_file_path(path: PathBuf) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let status = Command::new("open")
            .arg("-R")
            .arg(&path)
            .status()
            .map_err(|error| error.to_string())?;

        if status.success() {
            return Ok(());
        }
    }

    #[cfg(target_os = "windows")]
    {
        let status = Command::new("explorer")
            .arg(format!("/select,{}", path.display()))
            .status()
            .map_err(|error| error.to_string())?;

        if status.success() {
            return Ok(());
        }
    }

    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        let target = if path.is_dir() {
            path
        } else {
            path.parent()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."))
        };
        let status = Command::new("xdg-open")
            .arg(target)
            .status()
            .map_err(|error| error.to_string())?;

        if status.success() {
            return Ok(());
        }
    }

    Err("파일 위치를 열 수 없습니다.".to_string())
}
