use crate::hash_algorithm::HashAlgorithm;
use crate::scan_mode::ScanMode;
#[cfg(not(target_arch = "wasm32"))]
use jwalk::WalkDir;
#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use rusqlite::{params, Connection, OptionalExtension};
#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
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
    canonical_path: String,
    size: u64,
    modified_ns: i64,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
struct HashedFile {
    entry: FileEntry,
    hash: String,
    reused: bool,
}

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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DuplicateRelationKind {
    #[default]
    SameNameAndSize,
    SameSizeAndHash,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DuplicateRelation {
    pub original_path: String,
    pub duplicate_path: String,
    pub current_duplicate_path: String,
    pub size: u64,
    pub hash: String,
    pub kind: DuplicateRelationKind,
}

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
    if roots.is_empty() {
        return Err("검사할 디렉터리를 선택해야 합니다.".to_string());
    }

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

        if !root.exists() {
            return Err("입력한 경로가 존재하지 않습니다.".to_string());
        }

        if !root.is_dir() {
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
    let entries = collect_files_from_roots(&roots, &excluded_targets, &progress);
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
        let completed = index + 1;
        if index == 0 || index % 64 == 0 {
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
    let hashed_files: Vec<HashedFile> = missing
        .into_par_iter()
        .filter_map(|entry| {
            let completed = hashed_counter.fetch_add(1, Ordering::Relaxed) + 1;
            progress(ActivityEvent::with_progress(
                "해시 계산",
                entry.canonical_path.clone(),
                Some(entry.canonical_path.clone()),
                phase_progress(0.32, 0.64, completed, missing_total),
                completed,
                missing_total,
            ));
            algorithm
                .hash_path(&entry.path)
                .ok()
                .map(|hash| HashedFile {
                    entry,
                    hash,
                    reused: false,
                })
        })
        .collect();

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
    let deletion_result = delete_duplicate_groups(all_hashed, &quarantine_targets, &progress);

    let report = CleanReport {
        scanned_files: snapshot.scanned_files,
        candidate_files: snapshot.candidate_files,
        hashed_files: calculated_hashes,
        reused_hashes,
        duplicate_groups: deletion_result.duplicate_groups,
        deleted_files: deletion_result.deleted_files,
        kept_files: deletion_result.kept_files,
        reclaimed_bytes: deletion_result.reclaimed_bytes,
        failed_files: deletion_result.failed_files,
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
) -> Vec<FileEntry> {
    let mut by_volume = HashMap::<String, Vec<PathBuf>>::new();

    for root in roots {
        by_volume
            .entry(crate::quarantine::volume_key(root))
            .or_default()
            .push(root.clone());
    }

    let mut entries: Vec<FileEntry> = by_volume
        .into_values()
        .collect::<Vec<_>>()
        .into_par_iter()
        .flat_map(|group| {
            group
                .into_iter()
                .flat_map(|root| collect_files(&root, excluded_targets, progress))
                .collect::<Vec<_>>()
        })
        .collect();

    entries.sort_by(|left, right| left.canonical_path.cmp(&right.canonical_path));
    entries.dedup_by(|left, right| left.canonical_path == right.canonical_path);
    entries
}

#[cfg(not(target_arch = "wasm32"))]
fn collect_files(
    root: &Path,
    excluded_targets: &[PathBuf],
    progress: &(dyn Fn(ActivityEvent) + Sync),
) -> Vec<FileEntry> {
    let mut seen = 0_usize;
    WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_file() {
                return None;
            }

            seen += 1;
            if seen == 1 || seen % 64 == 0 {
                progress(ActivityEvent::new(
                    "파일 검색",
                    path.display().to_string(),
                    Some(path.display().to_string()),
                ));
            }

            let metadata = fs::metadata(&path).ok()?;
            if !metadata.is_file() {
                return None;
            }

            let canonical_path = fs::canonicalize(&path).ok()?.to_string_lossy().to_string();
            let canonical_path_buf = PathBuf::from(&canonical_path);

            if excluded_targets
                .iter()
                .any(|target| canonical_path_buf.starts_with(target))
            {
                return None;
            }

            let modified_ns = metadata
                .modified()
                .ok()
                .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_nanos().min(i64::MAX as u128) as i64)
                .unwrap_or_default();

            Some(FileEntry {
                path,
                canonical_path,
                size: metadata.len(),
                modified_ns,
            })
        })
        .collect()
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
            WHERE path = ?1 AND algorithm = ?2 AND size = ?3 AND modified_ns = ?4",
            params![
                entry.canonical_path,
                algorithm.id(),
                entry.size as i64,
                entry.modified_ns
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
            "INSERT INTO file_hashes(path, algorithm, size, modified_ns, hash, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, unixepoch())
                ON CONFLICT(path, algorithm) DO UPDATE SET
                    size = excluded.size,
                    modified_ns = excluded.modified_ns,
                    hash = excluded.hash,
                    updated_at = excluded.updated_at",
        )?;

        for file in files {
            statement.execute(params![
                file.entry.canonical_path,
                algorithm.id(),
                file.entry.size as i64,
                file.entry.modified_ns,
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
) -> DeletionResult {
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
        group.sort_by(|left, right| left.entry.canonical_path.cmp(&right.entry.canonical_path));
        let keep = group.remove(0);
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

    result
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
    let volume_key = crate::quarantine::volume_key(&entry.path);
    let target = quarantine_targets
        .get(&volume_key)
        .ok_or_else(|| "해당 디스크의 보관 폴더가 없습니다.".to_string())?;

    if volume_key != crate::quarantine::volume_key(target) {
        return Err("보관 폴더가 원본 파일과 같은 디스크에 있지 않습니다.".to_string());
    }

    fs::create_dir_all(target).map_err(|error| error.to_string())?;

    let destination = unique_destination(target, &entry.path);
    fs::rename(&entry.path, &destination).map_err(|error| error.to_string())?;
    Ok(destination)
}

#[cfg(not(target_arch = "wasm32"))]
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
