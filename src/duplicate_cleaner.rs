use crate::hash_algorithm::HashAlgorithm;
use crate::scan_mode::ScanMode;
#[cfg(not(target_arch = "wasm32"))]
use jwalk::WalkDir;
#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use rusqlite::{params, Connection, OptionalExtension};
#[cfg(not(target_arch = "wasm32"))]
use serde::Serialize;
#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(all(not(target_arch = "wasm32"), unix))]
use std::fs::OpenOptions;
#[cfg(not(target_arch = "wasm32"))]
use std::io::ErrorKind;
#[cfg(all(not(target_arch = "wasm32"), unix))]
use std::os::unix::fs::MetadataExt;
#[cfg(all(not(target_arch = "wasm32"), target_os = "windows"))]
use std::os::windows::fs::MetadataExt;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{AtomicUsize, Ordering};
#[cfg(not(target_arch = "wasm32"))]
use std::time::UNIX_EPOCH;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
struct FileEntry {
    path: PathBuf,
    root_key: String,
    canonical_path: String,
    size: u64,
    modified_ns: i64,
    fingerprint: FileFingerprint,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileFingerprint {
    size: u64,
    modified_ns: i64,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
    #[cfg(target_os = "windows")]
    file_attributes: u32,
    #[cfg(target_os = "windows")]
    creation_time: u64,
    #[cfg(target_os = "windows")]
    last_write_time: u64,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
struct HashedFile {
    entry: FileEntry,
    hash: String,
    reused: bool,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Default)]
struct CollectionResult {
    entries: Vec<FileEntry>,
    failed_files: Vec<String>,
}

#[cfg(not(target_arch = "wasm32"))]
enum MoveDestinationError {
    DestinationExists,
    Other(String),
}

#[cfg_attr(not(target_arch = "wasm32"), derive(Serialize))]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CleanReport {
    pub scanned_files: usize,
    pub candidate_files: usize,
    pub hashed_files: usize,
    pub reused_hashes: usize,
    pub duplicate_groups: usize,
    pub deleted_files: usize,
    pub kept_files: usize,
    pub reclaimed_bytes: u64,
    pub failed_files: Vec<String>,
    pub duplicate_relations: Vec<DuplicateRelation>,
}

#[cfg_attr(not(target_arch = "wasm32"), derive(Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DuplicateRelationKind {
    #[default]
    SameNameAndSize,
    SameSizeAndHash,
}

#[cfg_attr(not(target_arch = "wasm32"), derive(Serialize))]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DuplicateRelation {
    pub original_path: String,
    pub duplicate_path: String,
    pub current_duplicate_path: String,
    pub size: u64,
    pub hash: String,
    pub kind: DuplicateRelationKind,
}

#[cfg_attr(not(target_arch = "wasm32"), derive(Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct ActivityEvent {
    pub stage: String,
    pub detail: String,
    pub path: Option<String>,
    pub progress: Option<f64>,
    pub completed: Option<usize>,
    pub total: Option<usize>,
}

impl ActivityEvent {
    pub fn new(stage: impl Into<String>, detail: impl Into<String>, path: Option<String>) -> Self {
        Self {
            stage: stage.into(),
            detail: detail.into(),
            path,
            progress: None,
            completed: None,
            total: None,
        }
    }

    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub fn with_progress(
        stage: impl Into<String>,
        detail: impl Into<String>,
        path: Option<String>,
        progress: f64,
        completed: usize,
        total: usize,
    ) -> Self {
        Self {
            stage: stage.into(),
            detail: detail.into(),
            path,
            progress: Some(progress.clamp(0.0, 1.0)),
            completed: Some(completed),
            total: Some(total),
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn clean_duplicate_paths(
    _roots: Vec<PathBuf>,
    _algorithm: HashAlgorithm,
    _scan_mode: ScanMode,
) -> Result<CleanReport, String> {
    Err("웹 미리보기에서는 파일 시스템 검사를 실행할 수 없습니다.".to_string())
}

#[cfg(target_arch = "wasm32")]
pub fn clean_duplicate_paths_with_progress<F>(
    roots: Vec<PathBuf>,
    algorithm: HashAlgorithm,
    scan_mode: ScanMode,
    progress: F,
) -> Result<CleanReport, String>
where
    F: Fn(ActivityEvent),
{
    progress(ActivityEvent::new(
        "웹 미리보기",
        "파일 시스템 검사는 데스크톱 앱에서 실행됩니다.",
        None,
    ));
    clean_duplicate_paths(roots, algorithm, scan_mode)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn clean_duplicates(root: PathBuf) -> Result<CleanReport, String> {
    clean_duplicate_paths(vec![root], HashAlgorithm::Blake3, ScanMode::Fast)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn clean_duplicate_paths(
    roots: Vec<PathBuf>,
    algorithm: HashAlgorithm,
    scan_mode: ScanMode,
) -> Result<CleanReport, String> {
    clean_duplicate_paths_with_progress(roots, algorithm, scan_mode, |_| {})
}

#[cfg(not(target_arch = "wasm32"))]
pub fn clean_duplicate_paths_with_progress<F>(
    roots: Vec<PathBuf>,
    algorithm: HashAlgorithm,
    scan_mode: ScanMode,
    progress: F,
) -> Result<CleanReport, String>
where
    F: Fn(ActivityEvent) + Send + Sync,
{
    clean_duplicate_paths_with_progress_and_cancel(roots, algorithm, scan_mode, progress, || false)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn clean_duplicate_paths_with_progress_and_cancel<F, C>(
    roots: Vec<PathBuf>,
    algorithm: HashAlgorithm,
    scan_mode: ScanMode,
    progress: F,
    should_cancel: C,
) -> Result<CleanReport, String>
where
    F: Fn(ActivityEvent) + Send + Sync,
    C: Fn() -> bool + Send + Sync,
{
    if roots.is_empty() {
        return Err("검사할 디렉터리를 선택해야 합니다.".to_string());
    }

    ensure_not_cancelled(&should_cancel)?;

    progress(ActivityEvent::new(
        "검사 준비",
        format!("{}개 경로를 확인하는 중", roots.len()),
        None,
    ));

    for root in &roots {
        progress(ActivityEvent::new(
            "경로 확인",
            root.display().to_string(),
            Some(root.display().to_string()),
        ));

        let metadata = fs::symlink_metadata(root)
            .map_err(|_| "입력한 경로가 존재하지 않습니다.".to_string())?;

        if metadata.file_type().is_symlink() {
            return Err("심볼릭 링크 경로는 검사할 수 없습니다.".to_string());
        }

        if !metadata.is_dir() {
            return Err("디렉터리 경로를 입력해야 합니다.".to_string());
        }
    }

    let mut connection = crate::cache::open_connection()?;
    let quarantine_targets = crate::quarantine::build_target_map(&roots)?;
    let excluded_targets = crate::quarantine::canonical_targets(&quarantine_targets);
    let mut snapshot = CleanReport::default();
    crate::cache::save_run_snapshot(&connection, &roots, "검사 준비 중", &snapshot)?;

    progress(ActivityEvent::new(
        "파일 검색",
        "검사할 파일 목록을 수집하는 중",
        None,
    ));
    let collection = collect_files_from_roots(&roots, &excluded_targets, &progress);
    ensure_not_cancelled(&should_cancel)?;
    let entries = collection.entries;
    let mut failed_files = collection.failed_files;
    snapshot.scanned_files = entries.len();
    crate::cache::save_run_snapshot(&connection, &roots, "파일 목록 수집 완료", &snapshot)?;
    progress(ActivityEvent::with_progress(
        "파일 검색 완료",
        format!("{}개 파일을 찾았습니다.", snapshot.scanned_files),
        None,
        0.12,
        snapshot.scanned_files,
        snapshot.scanned_files.max(1),
    ));

    progress(ActivityEvent::new(
        "후보 선별",
        format!("{}개 파일에서 중복 후보를 찾는 중", entries.len()),
        None,
    ));
    let candidates = candidate_entries(entries, scan_mode);

    snapshot.candidate_files = candidates.len();
    crate::cache::save_run_snapshot(&connection, &roots, "중복 후보 선별 완료", &snapshot)?;
    let candidate_total = snapshot.candidate_files.max(1);
    progress(ActivityEvent::with_progress(
        "후보 선별 완료",
        format!("{}개 중복 후보를 선별했습니다.", snapshot.candidate_files),
        None,
        0.18,
        0,
        candidate_total,
    ));

    let mut cached = Vec::new();
    let mut missing = Vec::new();

    for (index, entry) in candidates.into_iter().enumerate() {
        ensure_not_cancelled(&should_cancel)?;
        let completed = index + 1;
        if index == 0 || index.is_multiple_of(64) {
            progress(ActivityEvent::with_progress(
                "캐시 확인",
                entry.canonical_path.clone(),
                Some(entry.canonical_path.clone()),
                phase_progress(0.18, 0.32, completed, candidate_total),
                completed,
                candidate_total,
            ));
        }

        if scan_mode.uses_cache() {
            match cached_hash(&connection, &entry, algorithm).map_err(|error| error.to_string())? {
                Some(hash) => cached.push(HashedFile {
                    entry,
                    hash,
                    reused: true,
                }),
                None => missing.push(entry),
            }
        } else {
            missing.push(entry);
        }
    }

    snapshot.reused_hashes = cached.len();
    crate::cache::save_run_snapshot(&connection, &roots, "캐시 확인 완료", &snapshot)?;
    let missing_total = missing.len().max(1);
    progress(ActivityEvent::with_progress(
        "캐시 확인 완료",
        format!(
            "{}개 캐시를 재사용하고 {}개 파일은 해시를 계산합니다.",
            snapshot.reused_hashes,
            missing.len()
        ),
        None,
        0.32,
        0,
        missing_total,
    ));

    let hashed_counter = AtomicUsize::new(0);
    let hash_results: Vec<Result<HashedFile, String>> = missing
        .into_par_iter()
        .filter_map(|entry| {
            if should_cancel() {
                return None;
            }

            let completed = hashed_counter.fetch_add(1, Ordering::Relaxed) + 1;
            progress(ActivityEvent::with_progress(
                "해시 계산",
                entry.canonical_path.clone(),
                Some(entry.canonical_path.clone()),
                phase_progress(0.32, 0.64, completed, missing_total),
                completed,
                missing_total,
            ));
            Some(match algorithm.hash_path(&entry.path) {
                Ok(hash) => Ok(HashedFile {
                    entry,
                    hash,
                    reused: false,
                }),
                Err(error) => Err(format!("{}: {}", entry.path.display(), error)),
            })
        })
        .collect();
    ensure_not_cancelled(&should_cancel)?;
    let mut hashed_files = Vec::new();
    for result in hash_results {
        match result {
            Ok(file) => hashed_files.push(file),
            Err(error) => failed_files.push(error),
        }
    }

    progress(ActivityEvent::with_progress(
        "해시 저장",
        format!("{}개 해시를 SQLite 캐시에 저장하는 중", hashed_files.len()),
        None,
        0.68,
        hashed_files.len(),
        hashed_files.len().max(1),
    ));
    save_hashes(&mut connection, &hashed_files, algorithm).map_err(|error| error.to_string())?;
    progress(ActivityEvent::with_progress(
        "해시 저장 완료",
        "SQLite 캐시에 해시 기록을 저장했습니다.",
        None,
        0.70,
        hashed_files.len(),
        hashed_files.len().max(1),
    ));
    progress(ActivityEvent::with_progress(
        "캐시 정리",
        "SQLite 해시 캐시 제한을 적용하는 중",
        None,
        0.72,
        0,
        1,
    ));
    crate::cache::prune_hash_cache(&connection)?;
    ensure_not_cancelled(&should_cancel)?;

    let mut all_hashed = cached;
    all_hashed.extend(hashed_files);

    let reused_hashes = all_hashed.iter().filter(|file| file.reused).count();
    let calculated_hashes = all_hashed.len().saturating_sub(reused_hashes);
    snapshot.hashed_files = calculated_hashes;
    snapshot.reused_hashes = reused_hashes;
    crate::cache::save_run_snapshot(&connection, &roots, "해시 계산 완료", &snapshot)?;
    progress(ActivityEvent::with_progress(
        "해시 계산 완료",
        format!(
            "{}개 해시 계산, {}개 캐시 재사용",
            calculated_hashes, reused_hashes
        ),
        None,
        0.74,
        all_hashed.len(),
        all_hashed.len().max(1),
    ));

    progress(ActivityEvent::with_progress(
        "중복 그룹 선별",
        format!("{}개 해시 결과에서 중복 그룹을 찾는 중", all_hashed.len()),
        None,
        0.78,
        0,
        all_hashed.len().max(1),
    ));
    let mut deletion_result =
        delete_duplicate_groups(all_hashed, &quarantine_targets, &progress, &should_cancel)?;
    failed_files.append(&mut deletion_result.failed_files);

    let report = CleanReport {
        scanned_files: snapshot.scanned_files,
        candidate_files: snapshot.candidate_files,
        hashed_files: calculated_hashes,
        reused_hashes,
        duplicate_groups: deletion_result.duplicate_groups,
        deleted_files: deletion_result.deleted_files,
        kept_files: deletion_result.kept_files,
        reclaimed_bytes: deletion_result.reclaimed_bytes,
        failed_files,
        duplicate_relations: deletion_result.duplicate_relations,
    };

    progress(ActivityEvent::with_progress(
        "결과 저장",
        "검사 결과를 SQLite 스냅샷에 저장하는 중",
        None,
        0.98,
        1,
        1,
    ));
    crate::cache::save_run_snapshot(&connection, &roots, "완료", &report)?;
    progress(ActivityEvent::with_progress(
        "결과 저장 완료",
        "검사 결과 저장을 완료했습니다.",
        None,
        1.0,
        1,
        1,
    ));

    Ok(report)
}

#[cfg(not(target_arch = "wasm32"))]
fn ensure_not_cancelled(should_cancel: &(dyn Fn() -> bool + Sync)) -> Result<(), String> {
    if should_cancel() {
        return Err("사용자가 검사를 중지했습니다.".to_string());
    }

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn candidate_entries(entries: Vec<FileEntry>, scan_mode: ScanMode) -> Vec<FileEntry> {
    if matches!(scan_mode, ScanMode::FullHash) {
        return entries;
    }

    let mut by_size = HashMap::<u64, Vec<FileEntry>>::new();

    for entry in entries {
        by_size.entry(entry.size).or_default().push(entry);
    }

    by_size
        .into_values()
        .filter(|group| group.len() > 1)
        .flatten()
        .collect()
}

#[cfg(not(target_arch = "wasm32"))]
fn collect_files_from_roots(
    roots: &[PathBuf],
    excluded_targets: &[PathBuf],
    progress: &(dyn Fn(ActivityEvent) + Sync),
) -> CollectionResult {
    let mut by_volume = HashMap::<String, Vec<PathBuf>>::new();

    for root in roots {
        by_volume
            .entry(crate::quarantine::volume_key(root))
            .or_default()
            .push(root.clone());
    }

    let mut results: Vec<CollectionResult> = by_volume
        .into_values()
        .collect::<Vec<_>>()
        .into_par_iter()
        .map(|group| {
            let mut result = CollectionResult::default();
            for root in group {
                let mut collected = collect_files(&root, excluded_targets, progress);
                result.entries.append(&mut collected.entries);
                result.failed_files.append(&mut collected.failed_files);
            }
            result
        })
        .collect();
    let mut entries = Vec::new();
    let mut failed_files = Vec::new();

    for result in &mut results {
        entries.append(&mut result.entries);
        failed_files.append(&mut result.failed_files);
    }

    entries.sort_by(|left, right| left.canonical_path.cmp(&right.canonical_path));
    entries.dedup_by(|left, right| left.canonical_path == right.canonical_path);
    failed_files.sort();
    failed_files.dedup();
    CollectionResult {
        entries,
        failed_files,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn collect_files(
    root: &Path,
    excluded_targets: &[PathBuf],
    progress: &(dyn Fn(ActivityEvent) + Sync),
) -> CollectionResult {
    let mut seen = 0_usize;
    let root_key = crate::quarantine::root_key(root);
    let mut result = CollectionResult::default();

    for item in WalkDir::new(root).follow_links(false).into_iter() {
        let entry = match item {
            Ok(entry) => entry,
            Err(error) => {
                result.failed_files.push(error.to_string());
                continue;
            }
        };
        let path = entry.path();
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) => {
                result
                    .failed_files
                    .push(format!("{}: {}", path.display(), error));
                continue;
            }
        };

        if metadata.file_type().is_symlink() || !metadata.is_file() {
            continue;
        }

        seen += 1;
        if seen == 1 || seen.is_multiple_of(64) {
            progress(ActivityEvent::new(
                "파일 검색",
                path.display().to_string(),
                Some(path.display().to_string()),
            ));
        }

        let canonical_path = match fs::canonicalize(&path) {
            Ok(path) => path.to_string_lossy().to_string(),
            Err(error) => {
                result
                    .failed_files
                    .push(format!("{}: {}", path.display(), error));
                continue;
            }
        };
        let canonical_path_buf = PathBuf::from(&canonical_path);

        if excluded_targets
            .iter()
            .any(|target| canonical_path_buf.starts_with(target))
        {
            continue;
        }

        let modified_ns = modified_ns(&metadata);

        result.entries.push(FileEntry {
            path,
            root_key: root_key.clone(),
            canonical_path,
            size: metadata.len(),
            modified_ns,
            fingerprint: FileFingerprint::from_metadata(&metadata, modified_ns),
        });
    }

    result
}

#[cfg(not(target_arch = "wasm32"))]
fn cached_hash(
    connection: &Connection,
    entry: &FileEntry,
    algorithm: HashAlgorithm,
) -> rusqlite::Result<Option<String>> {
    connection
        .query_row(
            "SELECT hash FROM file_hashes
            WHERE path = ?1 AND algorithm = ?2 AND size = ?3 AND modified_ns = ?4 AND fingerprint = ?5",
            params![
                entry.canonical_path,
                algorithm.id(),
                entry.size as i64,
                entry.modified_ns,
                entry.fingerprint.cache_key()
            ],
            |row| row.get(0),
        )
        .optional()
}

#[cfg(not(target_arch = "wasm32"))]
fn save_hashes(
    connection: &mut Connection,
    files: &[HashedFile],
    algorithm: HashAlgorithm,
) -> rusqlite::Result<()> {
    let transaction = connection.transaction()?;
    {
        let mut statement = transaction.prepare(
            "INSERT INTO file_hashes(path, algorithm, size, modified_ns, fingerprint, hash, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, unixepoch())
                ON CONFLICT(path, algorithm) DO UPDATE SET
                    size = excluded.size,
                    modified_ns = excluded.modified_ns,
                    fingerprint = excluded.fingerprint,
                    hash = excluded.hash,
                    updated_at = excluded.updated_at",
        )?;

        for file in files {
            statement.execute(params![
                file.entry.canonical_path,
                algorithm.id(),
                file.entry.size as i64,
                file.entry.modified_ns,
                file.entry.fingerprint.cache_key(),
                file.hash
            ])?;
        }
    }
    transaction.commit()
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Default)]
struct DeletionResult {
    duplicate_groups: usize,
    deleted_files: usize,
    kept_files: usize,
    reclaimed_bytes: u64,
    failed_files: Vec<String>,
    duplicate_relations: Vec<DuplicateRelation>,
}

#[cfg(not(target_arch = "wasm32"))]
fn delete_duplicate_groups(
    files: Vec<HashedFile>,
    quarantine_targets: &HashMap<String, PathBuf>,
    progress: &(dyn Fn(ActivityEvent) + Sync),
    should_cancel: &(dyn Fn() -> bool + Sync),
) -> Result<DeletionResult, String> {
    let mut by_hash = HashMap::<(u64, String), Vec<HashedFile>>::new();

    for file in files {
        by_hash
            .entry((file.entry.size, file.hash.clone()))
            .or_default()
            .push(file);
    }

    let groups: Vec<Vec<HashedFile>> = by_hash
        .into_values()
        .filter(|group| group.len() > 1)
        .collect();

    let mut result = DeletionResult {
        duplicate_groups: groups.len(),
        ..DeletionResult::default()
    };

    let group_total = groups.len().max(1);

    for (group_index, mut group) in groups.into_iter().enumerate() {
        ensure_not_cancelled(should_cancel)?;
        group.sort_by(|left, right| left.entry.canonical_path.cmp(&right.entry.canonical_path));
        let keep = group.remove(0);
        if let Err(error) = verify_file_fingerprint(&keep.entry) {
            result
                .failed_files
                .push(format!("{}: {}", keep.entry.path.display(), error));
            continue;
        }
        result.kept_files += 1;
        let group_completed = group_index + 1;

        progress(ActivityEvent::with_progress(
            "중복 그룹",
            format!("기준 파일 유지: {}", keep.entry.canonical_path),
            Some(keep.entry.canonical_path.clone()),
            phase_progress(0.78, 0.95, group_completed, group_total),
            group_completed,
            group_total,
        ));

        for duplicate in group {
            ensure_not_cancelled(should_cancel)?;
            let same_file =
                same_file::is_same_file(&keep.entry.path, &duplicate.entry.path).unwrap_or(false);

            if same_file {
                continue;
            }

            match move_to_quarantine(&duplicate.entry, quarantine_targets) {
                Ok(destination) => {
                    result.deleted_files += 1;
                    result.reclaimed_bytes += duplicate.entry.size;
                    result.duplicate_relations.push(DuplicateRelation {
                        original_path: keep.entry.canonical_path.clone(),
                        duplicate_path: duplicate.entry.canonical_path.clone(),
                        current_duplicate_path: destination.display().to_string(),
                        size: duplicate.entry.size,
                        hash: duplicate.hash.clone(),
                        kind: duplicate_relation_kind(&keep.entry.path, &duplicate.entry.path),
                    });
                    progress(ActivityEvent::with_progress(
                        "보관 이동",
                        duplicate.entry.canonical_path.clone(),
                        Some(duplicate.entry.canonical_path.clone()),
                        phase_progress(0.78, 0.95, group_completed, group_total),
                        group_completed,
                        group_total,
                    ));
                }
                Err(error) => result.failed_files.push(format!(
                    "{}: {}",
                    duplicate.entry.path.display(),
                    error
                )),
            }
        }
    }

    Ok(result)
}

#[cfg(not(target_arch = "wasm32"))]
fn phase_progress(start: f64, end: f64, completed: usize, total: usize) -> f64 {
    if total == 0 {
        return end;
    }

    start + (end - start) * (completed as f64 / total as f64)
}

#[cfg(not(target_arch = "wasm32"))]
fn duplicate_relation_kind(original: &Path, duplicate: &Path) -> DuplicateRelationKind {
    if original.file_name() == duplicate.file_name() {
        DuplicateRelationKind::SameNameAndSize
    } else {
        DuplicateRelationKind::SameSizeAndHash
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn move_to_quarantine(
    entry: &FileEntry,
    quarantine_targets: &HashMap<String, PathBuf>,
) -> Result<PathBuf, String> {
    let target = quarantine_targets
        .get(&entry.root_key)
        .ok_or_else(|| "해당 검사 폴더의 보관 폴더가 없습니다.".to_string())?;

    if crate::quarantine::volume_key(&entry.path) != crate::quarantine::volume_key(target) {
        return Err("보관 폴더가 원본 파일과 같은 디스크에 있지 않습니다.".to_string());
    }

    prepare_quarantine_target(target)?;
    for _ in 0..1024 {
        let destination = destination_for_move(target, &entry.path)?;
        if let Err(error) = verify_file_fingerprint(entry) {
            cleanup_reserved_destination(&destination);
            return Err(error);
        }
        match move_to_destination(entry, &destination) {
            Ok(()) => return Ok(destination),
            Err(MoveDestinationError::DestinationExists) => continue,
            Err(MoveDestinationError::Other(error)) => return Err(error),
        }
    }

    Err("사용 가능한 보관 파일명을 찾지 못했습니다.".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn prepare_quarantine_target(target: &Path) -> Result<(), String> {
    fs::create_dir_all(target).map_err(|error| error.to_string())?;
    let metadata = fs::symlink_metadata(target).map_err(|error| error.to_string())?;

    if metadata.file_type().is_symlink()
        || metadata_is_reparse_point(&metadata)
        || !metadata.is_dir()
    {
        return Err("보관 폴더는 심볼릭 링크가 아닌 디렉터리여야 합니다.".to_string());
    }

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn move_to_destination(entry: &FileEntry, destination: &Path) -> Result<(), MoveDestinationError> {
    match fs::rename(&entry.path, destination) {
        Ok(()) => Ok(()),
        Err(error) => {
            #[cfg(unix)]
            let _ = fs::remove_file(destination);
            if error.kind() == ErrorKind::AlreadyExists {
                Err(MoveDestinationError::DestinationExists)
            } else {
                Err(MoveDestinationError::Other(error.to_string()))
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn cleanup_reserved_destination(destination: &Path) {
    #[cfg(unix)]
    let _ = fs::remove_file(destination);

    #[cfg(not(unix))]
    let _ = destination;
}

#[cfg(not(target_arch = "wasm32"))]
fn verify_file_fingerprint(entry: &FileEntry) -> Result<(), String> {
    let metadata = fs::symlink_metadata(&entry.path).map_err(|error| error.to_string())?;

    if metadata.file_type().is_symlink()
        || metadata_is_reparse_point(&metadata)
        || !metadata.is_file()
    {
        return Err("중복 파일이 일반 파일이 아닙니다.".to_string());
    }

    let modified_ns = modified_ns(&metadata);
    let current = FileFingerprint::from_metadata(&metadata, modified_ns);

    if current != entry.fingerprint {
        return Err("중복 파일이 검사 이후 변경되어 이동하지 않았습니다.".to_string());
    }

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn modified_ns(metadata: &fs::Metadata) -> i64 {
    metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos().min(i64::MAX as u128) as i64)
        .unwrap_or_default()
}

#[cfg(all(not(target_arch = "wasm32"), target_os = "windows"))]
fn metadata_is_reparse_point(metadata: &fs::Metadata) -> bool {
    metadata.file_attributes()
        & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT
        != 0
}

#[cfg(all(not(target_arch = "wasm32"), not(target_os = "windows")))]
fn metadata_is_reparse_point(_metadata: &fs::Metadata) -> bool {
    false
}

#[cfg(not(target_arch = "wasm32"))]
impl FileFingerprint {
    fn from_metadata(metadata: &fs::Metadata, modified_ns: i64) -> Self {
        Self {
            size: metadata.len(),
            modified_ns,
            #[cfg(unix)]
            device: metadata.dev(),
            #[cfg(unix)]
            inode: metadata.ino(),
            #[cfg(target_os = "windows")]
            file_attributes: metadata.file_attributes(),
            #[cfg(target_os = "windows")]
            creation_time: metadata.creation_time(),
            #[cfg(target_os = "windows")]
            last_write_time: metadata.last_write_time(),
        }
    }

    fn cache_key(&self) -> String {
        #[cfg(unix)]
        {
            format!(
                "{}:{}:{}:{}",
                self.size, self.modified_ns, self.device, self.inode
            )
        }

        #[cfg(target_os = "windows")]
        {
            format!(
                "{}:{}:{}:{}:{}",
                self.size,
                self.modified_ns,
                self.file_attributes,
                self.creation_time,
                self.last_write_time
            )
        }

        #[cfg(all(not(unix), not(target_os = "windows")))]
        {
            format!("{}:{}", self.size, self.modified_ns)
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn destination_for_move(target: &Path, source: &Path) -> Result<PathBuf, String> {
    #[cfg(unix)]
    {
        reserve_destination(target, source)
    }

    #[cfg(not(unix))]
    {
        unique_available_destination(target, source)
    }
}

#[cfg(all(not(target_arch = "wasm32"), not(unix)))]
fn unique_available_destination(target: &Path, source: &Path) -> Result<PathBuf, String> {
    let destination = unique_destination(target, source);
    if destination.symlink_metadata().is_ok() {
        return Err("보관 위치에 같은 이름의 파일이 이미 있습니다.".to_string());
    }

    Ok(destination)
}

#[cfg(all(not(target_arch = "wasm32"), not(unix)))]
fn unique_destination(target: &Path, source: &Path) -> PathBuf {
    let file_name = source
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "duplicate-file".to_string());
    let mut index = 0_u64;

    loop {
        let candidate = if index == 0 {
            target.join(&file_name)
        } else {
            target.join(format!("{index}-{file_name}"))
        };

        if !candidate.exists() {
            return candidate;
        }

        index += 1;
    }
}

#[cfg(all(not(target_arch = "wasm32"), unix))]
fn reserve_destination(target: &Path, source: &Path) -> Result<PathBuf, String> {
    let file_name = source
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "duplicate-file".to_string());
    let mut index = 0_u64;

    loop {
        let candidate = if index == 0 {
            target.join(&file_name)
        } else {
            target.join(format!("{index}-{file_name}"))
        };

        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => {
                drop(file);
                return Ok(candidate);
            }
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                index += 1;
            }
            Err(error) => return Err(error.to_string()),
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("hash-killer-{name}-{}-{nonce}", std::process::id()))
    }

    fn write_file(path: &Path, content: &[u8]) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }

        fs::write(path, content).unwrap();
    }

    fn entry_for(path: &Path) -> FileEntry {
        let metadata = fs::symlink_metadata(path).unwrap();
        let modified_ns = modified_ns(&metadata);
        FileEntry {
            path: path.to_path_buf(),
            root_key: crate::quarantine::root_key(path.parent().unwrap_or(path)),
            canonical_path: fs::canonicalize(path).unwrap().display().to_string(),
            size: metadata.len(),
            modified_ns,
            fingerprint: FileFingerprint::from_metadata(&metadata, modified_ns),
        }
    }

    #[test]
    fn collect_files_excludes_quarantine_target() {
        let root = temp_path("exclude-quarantine");
        let quarantine = root.join("quarantine");
        let kept = root.join("kept.txt");
        let ignored = quarantine.join("ignored.txt");
        write_file(&kept, b"same");
        write_file(&ignored, b"same");
        let excluded = vec![fs::canonicalize(&quarantine).unwrap()];

        let files = collect_files(&root, &excluded, &|_| {});

        assert_eq!(files.entries.len(), 1);
        assert!(files.failed_files.is_empty());
        assert_eq!(
            files.entries[0].canonical_path,
            fs::canonicalize(&kept).unwrap().display().to_string()
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn collect_files_excludes_symlinks() {
        use std::os::unix::fs::symlink;

        let root = temp_path("exclude-symlink");
        let outside = temp_path("outside-symlink");
        let real = root.join("real.txt");
        let outside_file = outside.join("outside.txt");
        let symlink_file = root.join("linked-file.txt");
        let symlink_dir = root.join("linked-dir");
        write_file(&real, b"real");
        write_file(&outside_file, b"outside");
        symlink(&outside_file, &symlink_file).unwrap();
        symlink(&outside, &symlink_dir).unwrap();

        let files = collect_files(&root, &[], &|_| {});

        assert_eq!(files.entries.len(), 1);
        assert!(files.failed_files.is_empty());
        assert_eq!(
            files.entries[0].canonical_path,
            fs::canonicalize(&real).unwrap().display().to_string()
        );

        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }

    #[test]
    fn move_to_quarantine_rejects_changed_file() {
        let root = temp_path("changed-file");
        let quarantine = root.join("quarantine");
        let duplicate = root.join("duplicate.txt");
        write_file(&duplicate, b"before");
        let entry = entry_for(&duplicate);
        write_file(&duplicate, b"after-change");
        let mut targets = HashMap::new();
        targets.insert(entry.root_key.clone(), quarantine.clone());

        let result = move_to_quarantine(&entry, &targets);

        assert!(result.is_err());
        assert!(duplicate.exists());
        assert!(!quarantine.join("duplicate.txt").exists());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn move_to_quarantine_uses_numbered_destination_when_name_exists() {
        let root = temp_path("existing-destination");
        let quarantine = root.join("quarantine");
        let duplicate = root.join("duplicate.txt");
        let existing = quarantine.join("duplicate.txt");
        write_file(&duplicate, b"duplicate");
        write_file(&existing, b"existing");
        let entry = entry_for(&duplicate);
        let mut targets = HashMap::new();
        targets.insert(entry.root_key.clone(), quarantine.clone());

        let result = move_to_quarantine(&entry, &targets).unwrap();

        assert_eq!(result, quarantine.join("1-duplicate.txt"));
        assert!(!duplicate.exists());
        assert_eq!(fs::read(existing).unwrap(), b"existing");
        assert_eq!(fs::read(result).unwrap(), b"duplicate");

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cached_hash_requires_current_fingerprint() {
        let root = temp_path("cached-fingerprint");
        let file = root.join("file.txt");
        write_file(&file, b"same");
        let entry = entry_for(&file);
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE file_hashes (
                    path TEXT NOT NULL,
                    algorithm TEXT NOT NULL,
                    size INTEGER NOT NULL,
                    modified_ns INTEGER NOT NULL,
                    fingerprint TEXT NOT NULL,
                    hash TEXT NOT NULL,
                    updated_at INTEGER NOT NULL,
                    PRIMARY KEY(path, algorithm)
                );",
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO file_hashes(path, algorithm, size, modified_ns, fingerprint, hash, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1)",
                params![
                    entry.canonical_path,
                    HashAlgorithm::Blake3.id(),
                    entry.size as i64,
                    entry.modified_ns,
                    "stale",
                    "old-hash"
                ],
            )
            .unwrap();

        assert_eq!(
            cached_hash(&connection, &entry, HashAlgorithm::Blake3).unwrap(),
            None
        );

        connection
            .execute(
                "UPDATE file_hashes SET fingerprint = ?1 WHERE path = ?2 AND algorithm = ?3",
                params![
                    entry.fingerprint.cache_key(),
                    entry.canonical_path,
                    HashAlgorithm::Blake3.id()
                ],
            )
            .unwrap();

        assert_eq!(
            cached_hash(&connection, &entry, HashAlgorithm::Blake3).unwrap(),
            Some("old-hash".to_string())
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn delete_duplicate_groups_skips_group_when_keep_file_changed() {
        let root = temp_path("changed-keep");
        let quarantine = root.join("quarantine");
        let keep = root.join("a.txt");
        let duplicate = root.join("b.txt");
        write_file(&keep, b"same");
        write_file(&duplicate, b"same");
        let keep_entry = entry_for(&keep);
        let duplicate_entry = entry_for(&duplicate);
        fs::remove_file(&keep).unwrap();
        let mut targets = HashMap::new();
        targets.insert(keep_entry.root_key.clone(), quarantine);

        let result = delete_duplicate_groups(
            vec![
                HashedFile {
                    entry: keep_entry,
                    hash: "hash".to_string(),
                    reused: false,
                },
                HashedFile {
                    entry: duplicate_entry,
                    hash: "hash".to_string(),
                    reused: false,
                },
            ],
            &targets,
            &|_| {},
            &|| false,
        )
        .unwrap();

        assert_eq!(result.deleted_files, 0);
        assert_eq!(result.failed_files.len(), 1);
        assert!(duplicate.exists());

        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn move_to_quarantine_rejects_replaced_symlink() {
        use std::os::unix::fs::symlink;

        let root = temp_path("replaced-symlink");
        let outside = temp_path("outside-replaced-symlink");
        let quarantine = root.join("quarantine");
        let duplicate = root.join("duplicate.txt");
        let outside_file = outside.join("outside.txt");
        write_file(&duplicate, b"duplicate");
        write_file(&outside_file, b"duplicate");
        let entry = entry_for(&duplicate);
        fs::remove_file(&duplicate).unwrap();
        symlink(&outside_file, &duplicate).unwrap();
        let mut targets = HashMap::new();
        targets.insert(entry.root_key.clone(), quarantine.clone());

        let result = move_to_quarantine(&entry, &targets);

        assert!(result.is_err());
        assert!(outside_file.exists());
        assert!(duplicate.exists());
        assert!(!quarantine.join("duplicate.txt").exists());

        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn move_to_quarantine_rejects_symlink_target() {
        use std::os::unix::fs::symlink;

        let root = temp_path("symlink-target");
        let outside = temp_path("outside-symlink-target");
        let duplicate = root.join("duplicate.txt");
        let real_target = outside.join("real-target");
        let symlink_target = root.join("quarantine-link");
        write_file(&duplicate, b"duplicate");
        fs::create_dir_all(&real_target).unwrap();
        symlink(&real_target, &symlink_target).unwrap();
        let entry = entry_for(&duplicate);
        let mut targets = HashMap::new();
        targets.insert(entry.root_key.clone(), symlink_target.clone());

        let result = move_to_quarantine(&entry, &targets);

        assert!(result.is_err());
        assert!(duplicate.exists());
        assert!(!real_target.join("duplicate.txt").exists());

        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn delete_duplicate_groups_skips_hardlinked_duplicates() {
        let root = temp_path("hardlink");
        let quarantine = root.join("quarantine");
        let original = root.join("original.txt");
        let duplicate = root.join("duplicate.txt");
        write_file(&original, b"same");
        fs::hard_link(&original, &duplicate).unwrap();
        let hash = "hash".to_string();
        let files = vec![
            HashedFile {
                entry: entry_for(&original),
                hash: hash.clone(),
                reused: false,
            },
            HashedFile {
                entry: entry_for(&duplicate),
                hash,
                reused: false,
            },
        ];
        let mut targets = HashMap::new();
        targets.insert(crate::quarantine::root_key(&root), quarantine);

        let result = delete_duplicate_groups(files, &targets, &|_| {}, &|| false).unwrap();

        assert_eq!(result.deleted_files, 0);
        assert_eq!(result.reclaimed_bytes, 0);
        assert!(result.duplicate_relations.is_empty());

        fs::remove_dir_all(root).unwrap();
    }
}
