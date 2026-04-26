#[cfg(not(target_arch = "wasm32"))]
use crate::duplicate_cleaner::CleanReport;

#[cfg(not(target_arch = "wasm32"))]
use rusqlite::{params, Connection, OptionalExtension};
#[cfg(not(target_arch = "wasm32"))]
use std::env;
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
const DEFAULT_CACHE_LIMIT_MB: u64 = 256;
#[cfg(not(target_arch = "wasm32"))]
const CACHE_PRUNE_TARGET_RATIO: u64 = 80;
#[cfg(not(target_arch = "wasm32"))]
const STATE_DIR_ENV: &str = "HASH_KILLER_STATE_DIR";

#[cfg(target_arch = "wasm32")]
const DEFAULT_CACHE_LIMIT_MB: u64 = 256;

#[cfg(not(target_arch = "wasm32"))]
pub fn open_connection() -> Result<Connection, String> {
    let connection = Connection::open(cache_db_path()?).map_err(|error| error.to_string())?;
    connection
        .busy_timeout(Duration::from_secs(15))
        .map_err(|error| error.to_string())?;
    prepare_database(&connection).map_err(|error| error.to_string())?;
    Ok(connection)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn prune_hash_cache(connection: &Connection) -> Result<usize, String> {
    let limit = cache_limit_bytes(connection)?;
    connection
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .map_err(|error| error.to_string())?;
    let storage_bytes = cache_storage_bytes()?;

    if storage_bytes <= limit {
        return Ok(0);
    }

    let target = limit.saturating_mul(CACHE_PRUNE_TARGET_RATIO) / 100;
    let row_count = hash_cache_row_count(connection)?;

    if row_count == 0 {
        connection
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE); VACUUM;")
            .map_err(|error| error.to_string())?;
        return Ok(0);
    }

    let average_row_bytes = (storage_bytes / row_count).max(1);
    let target_rows = (target / average_row_bytes).min(row_count);
    let rows_to_remove = row_count.saturating_sub(target_rows).max(1);
    let removed = connection
        .execute(
            "DELETE FROM file_hashes
            WHERE rowid IN (
                SELECT rowid FROM file_hashes
                ORDER BY updated_at ASC, path ASC
                LIMIT ?1
            )",
            params![rows_to_remove as i64],
        )
        .map_err(|error| error.to_string())?;

    connection
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE); VACUUM;")
        .map_err(|error| error.to_string())?;

    Ok(removed)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_cache_limit_mb() -> Result<u64, String> {
    let connection = open_connection()?;
    cache_limit_mb(&connection)
}

#[cfg(target_arch = "wasm32")]
pub fn load_cache_limit_mb() -> Result<u64, String> {
    Ok(DEFAULT_CACHE_LIMIT_MB)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_cache_limit_configured() -> Result<bool, String> {
    let connection = open_connection()?;
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM app_settings WHERE key = 'cache_limit_mb')",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|value| value == 1)
        .map_err(|error| error.to_string())
}

#[cfg(target_arch = "wasm32")]
pub fn load_cache_limit_configured() -> Result<bool, String> {
    Ok(false)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_cache_limit_mb(value: u64) -> Result<usize, String> {
    if value < 16 {
        return Err("캐시 제한은 16 MB 이상으로 입력하십시오.".to_string());
    }

    let connection = open_connection()?;
    connection
        .execute(
            "INSERT INTO app_settings(key, value, updated_at)
            VALUES ('cache_limit_mb', ?1, unixepoch())
            ON CONFLICT(key) DO UPDATE SET
                value = excluded.value,
                updated_at = excluded.updated_at",
            params![value.to_string()],
        )
        .map_err(|error| error.to_string())?;

    prune_hash_cache(&connection)
}

#[cfg(target_arch = "wasm32")]
pub fn save_cache_limit_mb(_value: u64) -> Result<usize, String> {
    Err("웹 미리보기에서는 SQLite 캐시 설정을 저장할 수 없습니다.".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_hash_algorithm_id() -> Result<String, String> {
    let connection = open_connection()?;
    connection
        .query_row(
            "SELECT value FROM app_settings WHERE key = 'hash_algorithm'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map(|value| value.unwrap_or_else(|| "BLAKE3".to_string()))
        .map_err(|error| error.to_string())
}

#[cfg(target_arch = "wasm32")]
pub fn load_hash_algorithm_id() -> Result<String, String> {
    Ok("BLAKE3".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_hash_algorithm_configured() -> Result<bool, String> {
    let connection = open_connection()?;
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM app_settings WHERE key = 'hash_algorithm')",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|value| value == 1)
        .map_err(|error| error.to_string())
}

#[cfg(target_arch = "wasm32")]
pub fn load_hash_algorithm_configured() -> Result<bool, String> {
    Ok(false)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_hash_algorithm_id(value: &str) -> Result<(), String> {
    let connection = open_connection()?;
    connection
        .execute(
            "INSERT INTO app_settings(key, value, updated_at)
            VALUES ('hash_algorithm', ?1, unixepoch())
            ON CONFLICT(key) DO UPDATE SET
                value = excluded.value,
                updated_at = excluded.updated_at",
            params![value],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[cfg(target_arch = "wasm32")]
pub fn save_hash_algorithm_id(_value: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_scan_mode_id() -> Result<String, String> {
    let connection = open_connection()?;
    connection
        .query_row(
            "SELECT value FROM app_settings WHERE key = 'scan_mode'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map(|value| value.unwrap_or_else(|| "FAST".to_string()))
        .map_err(|error| error.to_string())
}

#[cfg(target_arch = "wasm32")]
pub fn load_scan_mode_id() -> Result<String, String> {
    Ok("FAST".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_scan_mode_configured() -> Result<bool, String> {
    let connection = open_connection()?;
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM app_settings WHERE key = 'scan_mode')",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|value| value == 1)
        .map_err(|error| error.to_string())
}

#[cfg(target_arch = "wasm32")]
pub fn load_scan_mode_configured() -> Result<bool, String> {
    Ok(false)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_scan_mode_id(value: &str) -> Result<(), String> {
    let connection = open_connection()?;
    connection
        .execute(
            "INSERT INTO app_settings(key, value, updated_at)
            VALUES ('scan_mode', ?1, unixepoch())
            ON CONFLICT(key) DO UPDATE SET
                value = excluded.value,
                updated_at = excluded.updated_at",
            params![value],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[cfg(target_arch = "wasm32")]
pub fn save_scan_mode_id(_value: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn clear_cache() -> Result<usize, String> {
    let db_path = cache_db_path()?;
    let mut removed = 0;

    for path in [
        db_path.clone(),
        db_path.with_extension("sqlite3-wal"),
        db_path.with_extension("sqlite3-shm"),
    ] {
        if !path.exists() {
            continue;
        }

        fs::remove_file(&path).map_err(|error| error.to_string())?;
        removed += 1;
    }

    Ok(removed)
}

#[cfg(target_arch = "wasm32")]
pub fn clear_cache() -> Result<usize, String> {
    Err("웹 미리보기에서는 SQLite 캐시를 삭제할 수 없습니다.".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_run_snapshot(
    connection: &Connection,
    roots: &[PathBuf],
    status: &str,
    report: &CleanReport,
) -> Result<(), String> {
    let roots = roots
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join("\n");

    connection
        .execute(
            "INSERT INTO scan_runs(
                id,
                roots,
                status,
                scanned_files,
                candidate_files,
                hashed_files,
                reused_hashes,
                duplicate_groups,
                deleted_files,
                kept_files,
                reclaimed_bytes,
                updated_at
            )
            VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, unixepoch())
            ON CONFLICT(id) DO UPDATE SET
                roots = excluded.roots,
                status = excluded.status,
                scanned_files = excluded.scanned_files,
                candidate_files = excluded.candidate_files,
                hashed_files = excluded.hashed_files,
                reused_hashes = excluded.reused_hashes,
                duplicate_groups = excluded.duplicate_groups,
                deleted_files = excluded.deleted_files,
                kept_files = excluded.kept_files,
                reclaimed_bytes = excluded.reclaimed_bytes,
                updated_at = excluded.updated_at",
            params![
                roots,
                status,
                report.scanned_files as i64,
                report.candidate_files as i64,
                report.hashed_files as i64,
                report.reused_hashes as i64,
                report.duplicate_groups as i64,
                report.deleted_files as i64,
                report.kept_files as i64,
                report.reclaimed_bytes as i64
            ],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn cache_db_path() -> Result<PathBuf, String> {
    let directory = state_directory()?;
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    Ok(directory.join("hash-killer.sqlite3"))
}

#[cfg(not(target_arch = "wasm32"))]
fn state_directory() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os(STATE_DIR_ENV) {
        return Ok(PathBuf::from(path));
    }

    platform_state_directory()
}

#[cfg(all(not(target_arch = "wasm32"), target_os = "windows"))]
fn platform_state_directory() -> Result<PathBuf, String> {
    env::var_os("APPDATA")
        .map(PathBuf::from)
        .map(|path| path.join("hash-killer"))
        .ok_or_else(|| "APPDATA 환경 변수를 찾을 수 없습니다.".to_string())
}

#[cfg(all(not(target_arch = "wasm32"), target_os = "macos"))]
fn platform_state_directory() -> Result<PathBuf, String> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .map(|path| {
            path.join("Library")
                .join("Application Support")
                .join("hash-killer")
        })
        .ok_or_else(|| "HOME 환경 변수를 찾을 수 없습니다.".to_string())
}

#[cfg(all(
    not(target_arch = "wasm32"),
    not(target_os = "windows"),
    not(target_os = "macos")
))]
fn platform_state_directory() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("XDG_DATA_HOME") {
        return Ok(PathBuf::from(path).join("hash-killer"));
    }

    env::var_os("HOME")
        .map(PathBuf::from)
        .map(|path| path.join(".local").join("share").join("hash-killer"))
        .ok_or_else(|| "HOME 환경 변수를 찾을 수 없습니다.".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn cache_limit_mb(connection: &Connection) -> Result<u64, String> {
    let stored = connection
        .query_row(
            "SELECT value FROM app_settings WHERE key = 'cache_limit_mb'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;

    Ok(stored
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value >= 16)
        .unwrap_or(DEFAULT_CACHE_LIMIT_MB))
}

#[cfg(not(target_arch = "wasm32"))]
fn cache_limit_bytes(connection: &Connection) -> Result<u64, String> {
    Ok(cache_limit_mb(connection)?.saturating_mul(1024 * 1024))
}

#[cfg(not(target_arch = "wasm32"))]
fn cache_storage_bytes() -> Result<u64, String> {
    let db_path = cache_db_path()?;
    let paths = [
        db_path.clone(),
        db_path.with_extension("sqlite3-wal"),
        db_path.with_extension("sqlite3-shm"),
    ];

    paths.iter().try_fold(0_u64, |total, path| {
        if !path.exists() {
            return Ok(total);
        }

        fs::metadata(path)
            .map(|metadata| total.saturating_add(metadata.len()))
            .map_err(|error| error.to_string())
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn hash_cache_row_count(connection: &Connection) -> Result<u64, String> {
    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM file_hashes", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;

    Ok(count as u64)
}

#[cfg(not(target_arch = "wasm32"))]
fn prepare_database(connection: &Connection) -> rusqlite::Result<()> {
    connection.pragma_update(None, "journal_mode", "WAL")?;
    connection.pragma_update(None, "synchronous", "NORMAL")?;
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS scan_runs (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                roots TEXT NOT NULL,
                status TEXT NOT NULL,
                scanned_files INTEGER NOT NULL,
                candidate_files INTEGER NOT NULL,
                hashed_files INTEGER NOT NULL,
                reused_hashes INTEGER NOT NULL,
                duplicate_groups INTEGER NOT NULL,
                deleted_files INTEGER NOT NULL,
                kept_files INTEGER NOT NULL,
                reclaimed_bytes INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );",
    )?;
    ensure_file_hashes_schema(connection)?;
    connection.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_file_hashes_algorithm_size_hash
            ON file_hashes(algorithm, size, hash);",
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn ensure_file_hashes_schema(connection: &Connection) -> rusqlite::Result<()> {
    if !table_exists(connection, "file_hashes")? {
        create_file_hashes_table(connection)?;
        return Ok(());
    }

    if file_hashes_has_algorithm(connection)? {
        return Ok(());
    }

    connection.execute_batch(
        "ALTER TABLE file_hashes RENAME TO file_hashes_legacy;
        CREATE TABLE file_hashes (
            path TEXT NOT NULL,
            algorithm TEXT NOT NULL,
            size INTEGER NOT NULL,
            modified_ns INTEGER NOT NULL,
            hash TEXT NOT NULL,
            updated_at INTEGER NOT NULL,
            PRIMARY KEY(path, algorithm)
        );
        INSERT OR IGNORE INTO file_hashes(path, algorithm, size, modified_ns, hash, updated_at)
            SELECT path, 'BLAKE3', size, modified_ns, hash, updated_at
            FROM file_hashes_legacy;
        DROP TABLE file_hashes_legacy;",
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn create_file_hashes_table(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(
        "CREATE TABLE file_hashes (
            path TEXT NOT NULL,
            algorithm TEXT NOT NULL,
            size INTEGER NOT NULL,
            modified_ns INTEGER NOT NULL,
            hash TEXT NOT NULL,
            updated_at INTEGER NOT NULL,
            PRIMARY KEY(path, algorithm)
        );",
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn table_exists(connection: &Connection, table: &str) -> rusqlite::Result<bool> {
    connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
            params![table],
            |_| Ok(()),
        )
        .optional()
        .map(|value| value.is_some())
}

#[cfg(not(target_arch = "wasm32"))]
fn file_hashes_has_algorithm(connection: &Connection) -> rusqlite::Result<bool> {
    let mut statement = connection.prepare("PRAGMA table_info(file_hashes)")?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;

    for column in columns {
        if column? == "algorithm" {
            return Ok(true);
        }
    }

    Ok(false)
}
