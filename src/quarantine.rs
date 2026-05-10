#[cfg(not(target_arch = "wasm32"))]
use serde::Serialize;
#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

#[cfg_attr(not(target_arch = "wasm32"), derive(Serialize))]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VolumeDestination {
    pub volume_key: String,
    pub root_path: String,
    pub root_paths: Vec<String>,
    pub target_path: String,
    pub configured: bool,
}

#[cfg(not(target_arch = "wasm32"))]
use rusqlite::params;
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(all(not(target_arch = "wasm32"), unix))]
use std::os::unix::fs::MetadataExt;

#[cfg(not(target_arch = "wasm32"))]
const SETTING_PREFIX: &str = "quarantine_target:";

#[cfg(not(target_arch = "wasm32"))]
pub fn volume_destinations(roots: &[String]) -> Result<Vec<VolumeDestination>, String> {
    let configured = load_target_map()?;
    let mut root_paths = roots.iter().map(PathBuf::from).collect::<Vec<_>>();
    root_paths.sort();
    root_paths.dedup();

    let mut destinations = root_paths
        .into_iter()
        .map(|root_path| {
            let target_root_key = root_key(&root_path);
            let configured_path = configured.get(&target_root_key).cloned();
            let valid_configured = configured_path
                .as_ref()
                .is_some_and(|path| volume_key(&root_path) == volume_key(path));
            let target_path = configured_path
                .filter(|_| valid_configured)
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "지정되지 않음".to_string());
            let root_path = root_path.display().to_string();

            VolumeDestination {
                volume_key: target_root_key,
                root_path: root_path.clone(),
                root_paths: vec![root_path],
                target_path,
                configured: valid_configured,
            }
        })
        .collect::<Vec<_>>();

    destinations.sort_by(|left, right| left.root_path.cmp(&right.root_path));
    Ok(destinations)
}

#[cfg(target_arch = "wasm32")]
pub fn volume_destinations(_roots: &[String]) -> Result<Vec<VolumeDestination>, String> {
    Ok(Vec::new())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_destination(target_volume_key: &str, target_path: &Path) -> Result<(), String> {
    let root_path = PathBuf::from(target_volume_key);

    if volume_key(&root_path) != volume_key(target_path) {
        return Err("보관 폴더는 해당 검사 경로와 같은 디스크에 있어야 합니다.".to_string());
    }

    fs::create_dir_all(target_path).map_err(|error| error.to_string())?;

    let connection = crate::cache::open_connection()?;
    connection
        .execute(
            "INSERT INTO app_settings(key, value, updated_at)
            VALUES (?1, ?2, unixepoch())
            ON CONFLICT(key) DO UPDATE SET
                value = excluded.value,
                updated_at = excluded.updated_at",
            params![
                setting_key(target_volume_key),
                target_path.display().to_string()
            ],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn clear_destinations() -> Result<usize, String> {
    let connection = crate::cache::open_connection()?;
    connection
        .execute(
            "DELETE FROM app_settings WHERE key LIKE ?1",
            params![format!("{SETTING_PREFIX}%")],
        )
        .map_err(|error| error.to_string())
}

#[cfg(target_arch = "wasm32")]
pub fn clear_destinations() -> Result<usize, String> {
    Ok(0)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn build_target_map(roots: &[PathBuf]) -> Result<HashMap<String, PathBuf>, String> {
    let root_strings = roots
        .iter()
        .map(|root| root.display().to_string())
        .collect::<Vec<_>>();
    let destinations = volume_destinations(&root_strings)?;
    let mut targets = HashMap::new();

    for destination in destinations {
        if !destination.configured {
            return Err("모든 검사 폴더의 보관 폴더를 먼저 지정하십시오.".to_string());
        }

        let target = PathBuf::from(destination.target_path);

        if volume_key(Path::new(&destination.volume_key)) != volume_key(&target) {
            return Err("보관 폴더는 검사 경로와 같은 디스크에 있어야 합니다.".to_string());
        }

        fs::create_dir_all(&target).map_err(|error| error.to_string())?;
        targets.insert(destination.volume_key, target);
    }

    Ok(targets)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn canonical_targets(targets: &HashMap<String, PathBuf>) -> Vec<PathBuf> {
    targets
        .values()
        .filter_map(|path| fs::canonicalize(path).ok())
        .collect()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn volume_key(path: &Path) -> String {
    let probe = path
        .ancestors()
        .find(|ancestor| ancestor.exists())
        .unwrap_or(path);

    platform_volume_key(probe)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn root_key(path: &Path) -> String {
    fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string()
}

#[cfg(all(not(target_arch = "wasm32"), unix))]
fn platform_volume_key(path: &Path) -> String {
    fs::metadata(path)
        .map(|metadata| metadata.dev().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

#[cfg(all(not(target_arch = "wasm32"), not(unix)))]
fn platform_volume_key(path: &Path) -> String {
    path.components()
        .next()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn load_target_map() -> Result<HashMap<String, PathBuf>, String> {
    let connection = crate::cache::open_connection()?;
    let mut statement = connection
        .prepare("SELECT key, value FROM app_settings WHERE key LIKE ?1")
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(params![format!("{SETTING_PREFIX}%")], |row| {
            let key: String = row.get(0)?;
            let value: String = row.get(1)?;
            Ok((key, value))
        })
        .map_err(|error| error.to_string())?;
    let mut targets = HashMap::new();

    for row in rows {
        let (key, value) = row.map_err(|error| error.to_string())?;

        if let Some(volume_key) = key.strip_prefix(SETTING_PREFIX) {
            targets.insert(volume_key.to_string(), PathBuf::from(value));
        }
    }

    Ok(targets)
}

#[cfg(not(target_arch = "wasm32"))]
fn setting_key(volume_key: &str) -> String {
    format!("{SETTING_PREFIX}{volume_key}")
}
