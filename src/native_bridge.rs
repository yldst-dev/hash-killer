use crate::duplicate_cleaner::{
    ActivityEvent, CleanReport, DuplicateRelation, DuplicateRelationKind,
};
use crate::hash_algorithm::HashAlgorithm;
use crate::quarantine::VolumeDestination;
use crate::scan_mode::ScanMode;
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;

#[derive(Deserialize)]
struct BridgeRequest {
    command: String,
    roots: Option<Vec<String>>,
    algorithm: Option<String>,
    scan_mode: Option<String>,
    value: Option<serde_json::Value>,
    volume_key: Option<String>,
    target_path: Option<String>,
    path: Option<String>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum BridgeOutput<T>
where
    T: Serialize,
{
    #[serde(rename = "result")]
    Result { data: T },
    #[serde(rename = "activity")]
    Activity { event: ActivityEventDto },
    #[serde(rename = "report")]
    Report { report: CleanReportDto },
    #[serde(rename = "error")]
    Error { message: String },
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

#[derive(Serialize)]
struct ActivityEventDto {
    stage: String,
    detail: String,
    path: Option<String>,
    progress: Option<f64>,
    completed: Option<usize>,
    total: Option<usize>,
}

#[derive(Serialize)]
struct CleanReportDto {
    scanned_files: usize,
    candidate_files: usize,
    hashed_files: usize,
    reused_hashes: usize,
    duplicate_groups: usize,
    deleted_files: usize,
    kept_files: usize,
    reclaimed_bytes: u64,
    failed_files: Vec<String>,
    duplicate_relations: Vec<DuplicateRelationDto>,
}

#[derive(Serialize)]
struct DuplicateRelationDto {
    original_path: String,
    duplicate_path: String,
    current_duplicate_path: String,
    size: u64,
    hash: String,
    kind: String,
}

#[derive(Serialize)]
struct VolumeDestinationDto {
    volume_key: String,
    root_path: String,
    root_paths: Vec<String>,
    target_path: String,
    configured: bool,
}

pub fn run_stdio() -> i32 {
    match run_stdio_inner() {
        Ok(()) => 0,
        Err(error) => {
            let output = BridgeOutput::<UnitDto>::Error { message: error };
            let _ = write_json_line(&mut io::stdout(), &output);
            1
        }
    }
}

fn run_stdio_inner() -> Result<(), String> {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|error| error.to_string())?;
    let request: BridgeRequest = serde_json::from_str(&input).map_err(|error| error.to_string())?;

    match request.command.as_str() {
        "run_scan" => run_scan(request),
        "load_settings" => write_result(load_settings()?),
        "save_cache_limit" => {
            let value = request.u64_value("value")?;
            let pruned = crate::cache::save_cache_limit_mb(value)?;
            write_result(CacheSaveDto { pruned })
        }
        "clear_cache" => {
            let removed = crate::cache::clear_cache()?;
            write_result(CacheClearDto { removed })
        }
        "save_hash_algorithm" => {
            let value = request.string_value("value")?;
            let algorithm = HashAlgorithm::from_id(&value);
            crate::cache::save_hash_algorithm_id(algorithm.id())?;
            write_result(UnitDto { ok: true })
        }
        "save_scan_mode" => {
            let value = request.string_value("value")?;
            let mode = ScanMode::from_id(&value);
            crate::cache::save_scan_mode_id(mode.id())?;
            write_result(UnitDto { ok: true })
        }
        "volume_destinations" => {
            let roots = request.roots.unwrap_or_default();
            let destinations = crate::quarantine::volume_destinations(&roots)?
                .into_iter()
                .map(VolumeDestinationDto::from)
                .collect::<Vec<_>>();
            write_result(destinations)
        }
        "save_quarantine_destination" => {
            let volume_key = request
                .volume_key
                .ok_or_else(|| "volume_key 값이 필요합니다.".to_string())?;
            let target_path = request
                .target_path
                .ok_or_else(|| "target_path 값이 필요합니다.".to_string())?;
            crate::quarantine::save_destination(&volume_key, &PathBuf::from(target_path))?;
            write_result(UnitDto { ok: true })
        }
        "clear_quarantine_destinations" => {
            let count = crate::quarantine::clear_destinations()?;
            write_result(CountDto { count })
        }
        "reveal_file" => {
            let path = request
                .path
                .ok_or_else(|| "path 값이 필요합니다.".to_string())?;
            reveal_file(PathBuf::from(path))?;
            write_result(UnitDto { ok: true })
        }
        _ => Err(format!(
            "지원하지 않는 브리지 명령입니다: {}",
            request.command
        )),
    }
}

fn run_scan(request: BridgeRequest) -> Result<(), String> {
    let roots = request
        .roots
        .unwrap_or_default()
        .into_iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    let algorithm = HashAlgorithm::from_id(request.algorithm.as_deref().unwrap_or("BLAKE3"));
    let scan_mode = ScanMode::from_id(request.scan_mode.as_deref().unwrap_or("FAST"));
    let output = Mutex::new(io::stdout());
    let report = crate::duplicate_cleaner::clean_duplicate_paths_with_progress(
        roots,
        algorithm,
        scan_mode,
        |event| {
            let payload = BridgeOutput::<UnitDto>::Activity {
                event: ActivityEventDto::from(event),
            };

            if let Ok(mut writer) = output.lock() {
                let _ = write_json_line(&mut *writer, &payload);
            }
        },
    )?;
    let payload = BridgeOutput::<UnitDto>::Report {
        report: CleanReportDto::from(report),
    };
    let mut writer = output.lock().map_err(|error| error.to_string())?;
    write_json_line(&mut *writer, &payload)
}

fn load_settings() -> Result<SettingsDto, String> {
    Ok(SettingsDto {
        cache_limit_mb: crate::cache::load_cache_limit_mb()?,
        cache_limit_configured: crate::cache::load_cache_limit_configured()?,
        algorithm: crate::cache::load_hash_algorithm_id()?,
        algorithm_configured: crate::cache::load_hash_algorithm_configured()?,
        scan_mode: crate::cache::load_scan_mode_id()?,
        scan_mode_configured: crate::cache::load_scan_mode_configured()?,
    })
}

fn write_result<T>(data: T) -> Result<(), String>
where
    T: Serialize,
{
    let payload = BridgeOutput::Result { data };
    write_json_line(&mut io::stdout(), &payload)
}

fn write_json_line<T>(writer: &mut impl Write, value: &T) -> Result<(), String>
where
    T: Serialize,
{
    serde_json::to_writer(&mut *writer, value).map_err(|error| error.to_string())?;
    writer.write_all(b"\n").map_err(|error| error.to_string())?;
    writer.flush().map_err(|error| error.to_string())
}

fn reveal_file(path: PathBuf) -> Result<(), String> {
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

impl BridgeRequest {
    fn string_value(&self, field: &str) -> Result<String, String> {
        self.value
            .as_ref()
            .and_then(|value| value.as_str())
            .map(ToOwned::to_owned)
            .ok_or_else(|| format!("{field} 값이 필요합니다."))
    }

    fn u64_value(&self, field: &str) -> Result<u64, String> {
        self.value
            .as_ref()
            .and_then(|value| {
                value
                    .as_u64()
                    .or_else(|| value.as_str().and_then(|text| text.parse::<u64>().ok()))
            })
            .ok_or_else(|| format!("{field} 값이 필요합니다."))
    }
}

impl From<ActivityEvent> for ActivityEventDto {
    fn from(event: ActivityEvent) -> Self {
        Self {
            stage: event.stage,
            detail: event.detail,
            path: event.path,
            progress: event.progress,
            completed: event.completed,
            total: event.total,
        }
    }
}

impl From<CleanReport> for CleanReportDto {
    fn from(report: CleanReport) -> Self {
        Self {
            scanned_files: report.scanned_files,
            candidate_files: report.candidate_files,
            hashed_files: report.hashed_files,
            reused_hashes: report.reused_hashes,
            duplicate_groups: report.duplicate_groups,
            deleted_files: report.deleted_files,
            kept_files: report.kept_files,
            reclaimed_bytes: report.reclaimed_bytes,
            failed_files: report.failed_files,
            duplicate_relations: report
                .duplicate_relations
                .into_iter()
                .map(DuplicateRelationDto::from)
                .collect(),
        }
    }
}

impl From<DuplicateRelation> for DuplicateRelationDto {
    fn from(relation: DuplicateRelation) -> Self {
        Self {
            original_path: relation.original_path,
            duplicate_path: relation.duplicate_path,
            current_duplicate_path: relation.current_duplicate_path,
            size: relation.size,
            hash: relation.hash,
            kind: match relation.kind {
                DuplicateRelationKind::SameNameAndSize => "SAME_NAME_AND_SIZE",
                DuplicateRelationKind::SameSizeAndHash => "SAME_SIZE_AND_HASH",
            }
            .to_string(),
        }
    }
}

impl From<VolumeDestination> for VolumeDestinationDto {
    fn from(destination: VolumeDestination) -> Self {
        Self {
            volume_key: destination.volume_key,
            root_path: destination.root_path,
            root_paths: destination.root_paths,
            target_path: destination.target_path,
            configured: destination.configured,
        }
    }
}
