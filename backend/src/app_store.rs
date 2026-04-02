use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::config::get_home_dir;
use crate::error::AppError;

const STORE_KEY_APP_CONFIG_DIR: &str = "app_config_dir_override";
const STORE_FILE_NAME: &str = "app_paths.json";

#[derive(Debug, Default, Deserialize, Serialize)]
struct AppPathStore {
    #[serde(default)]
    app_config_dir_override: Option<String>,
}

fn store_file_path() -> PathBuf {
    crate::config::get_home_dir()
        .join(".cc-switch")
        .join(STORE_FILE_NAME)
}

fn resolve_path(raw: &str) -> PathBuf {
    let home = get_home_dir();
    if raw == "~" {
        return home;
    } else if let Some(stripped) = raw.strip_prefix("~/") {
        return home.join(stripped);
    } else if let Some(stripped) = raw.strip_prefix("~\\") {
        return home.join(stripped);
    }

    PathBuf::from(raw)
}

fn read_store() -> Option<AppPathStore> {
    let path = store_file_path();
    if !path.exists() {
        return None;
    }

    let raw = std::fs::read_to_string(&path).ok()?;
    match serde_json::from_str::<AppPathStore>(&raw) {
        Ok(store) => Some(store),
        Err(error) => {
            log::warn!("无法解析 {}: {error}", path.display());
            None
        }
    }
}

pub fn get_app_config_dir_override() -> Option<PathBuf> {
    let path_str = read_store()?.app_config_dir_override?;
    let trimmed = path_str.trim();
    if trimmed.is_empty() {
        return None;
    }

    let path = resolve_path(trimmed);
    if !path.exists() {
        log::warn!(
            "{} 中配置的 {STORE_KEY_APP_CONFIG_DIR} 不存在: {}",
            store_file_path().display(),
            path.display()
        );
        return None;
    }

    Some(path)
}

pub fn refresh_app_config_dir_override() -> Option<PathBuf> {
    get_app_config_dir_override()
}

pub fn set_app_config_dir_override(path: Option<&str>) -> Result<(), AppError> {
    let store_path = store_file_path();
    if let Some(parent) = store_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }

    let mut store = read_store().unwrap_or_default();
    store.app_config_dir_override = path
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    let json = serde_json::to_string_pretty(&store)
        .map_err(|e| AppError::Message(format!("序列化 app path store 失败: {e}")))?;
    std::fs::write(&store_path, json).map_err(|e| AppError::io(&store_path, e))?;

    Ok(())
}
