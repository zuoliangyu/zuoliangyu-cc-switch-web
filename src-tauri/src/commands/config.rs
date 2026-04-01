#![allow(non_snake_case)]

use crate::app_config::AppType;
use crate::codex_config;
use crate::config::{self, ConfigStatus};
use crate::settings;

#[tauri::command]
pub async fn get_claude_config_status() -> Result<ConfigStatus, String> {
    Ok(config::get_claude_config_status())
}

use std::str::FromStr;

pub(crate) enum ConfigCommandError {
    BadRequest(String),
    Internal(String),
}

impl ConfigCommandError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest(message.into())
    }

    fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }

    fn into_string(self) -> String {
        match self {
            Self::BadRequest(message) | Self::Internal(message) => message,
        }
    }
}

pub(crate) fn invalid_json_format_error(error: serde_json::Error) -> String {
    let lang = settings::get_settings()
        .language
        .unwrap_or_else(|| "zh".to_string());

    match lang.as_str() {
        "en" => format!("Invalid JSON format: {error}"),
        "ja" => format!("JSON形式が無効です: {error}"),
        _ => format!("无效的 JSON 格式: {error}"),
    }
}

fn invalid_toml_format_error(error: toml_edit::TomlError) -> String {
    let lang = settings::get_settings()
        .language
        .unwrap_or_else(|| "zh".to_string());

    match lang.as_str() {
        "en" => format!("Invalid TOML format: {error}"),
        "ja" => format!("TOML形式が無効です: {error}"),
        _ => format!("无效的 TOML 格式: {error}"),
    }
}

pub(crate) fn validate_common_config_snippet(app_type: &str, snippet: &str) -> Result<(), String> {
    if snippet.trim().is_empty() {
        return Ok(());
    }

    match app_type {
        "claude" | "gemini" | "omo" | "omo-slim" => {
            serde_json::from_str::<serde_json::Value>(snippet)
                .map_err(invalid_json_format_error)?;
        }
        "codex" => {
            snippet
                .parse::<toml_edit::DocumentMut>()
                .map_err(invalid_toml_format_error)?;
        }
        _ => {}
    }

    Ok(())
}

#[tauri::command]
pub async fn get_config_status(app: String) -> Result<ConfigStatus, String> {
    match AppType::from_str(&app).map_err(|e| e.to_string())? {
        AppType::Claude => Ok(config::get_claude_config_status()),
        AppType::Codex => {
            let auth_path = codex_config::get_codex_auth_path();
            let exists = auth_path.exists();
            let path = codex_config::get_codex_config_dir()
                .to_string_lossy()
                .to_string();

            Ok(ConfigStatus { exists, path })
        }
        AppType::Gemini => {
            let env_path = crate::gemini_config::get_gemini_env_path();
            let exists = env_path.exists();
            let path = crate::gemini_config::get_gemini_dir()
                .to_string_lossy()
                .to_string();

            Ok(ConfigStatus { exists, path })
        }
        AppType::OpenCode => {
            let config_path = crate::opencode_config::get_opencode_config_path();
            let exists = config_path.exists();
            let path = crate::opencode_config::get_opencode_dir()
                .to_string_lossy()
                .to_string();

            Ok(ConfigStatus { exists, path })
        }
        AppType::OpenClaw => {
            let config_path = crate::openclaw_config::get_openclaw_config_path();
            let exists = config_path.exists();
            let path = crate::openclaw_config::get_openclaw_dir()
                .to_string_lossy()
                .to_string();

            Ok(ConfigStatus { exists, path })
        }
    }
}

#[tauri::command]
pub async fn get_config_dir(app: String) -> Result<String, String> {
    get_config_dir_internal(app)
}

pub(crate) fn get_config_dir_internal(app: String) -> Result<String, String> {
    let dir = match AppType::from_str(&app).map_err(|e| e.to_string())? {
        AppType::Claude => config::get_claude_config_dir(),
        AppType::Codex => codex_config::get_codex_config_dir(),
        AppType::Gemini => crate::gemini_config::get_gemini_dir(),
        AppType::OpenCode => crate::opencode_config::get_opencode_dir(),
        AppType::OpenClaw => crate::openclaw_config::get_openclaw_dir(),
    };

    Ok(dir.to_string_lossy().to_string())
}

pub(crate) fn get_common_config_snippet_internal(
    state: &crate::store::AppState,
    app_type: &str,
) -> Result<Option<String>, String> {
    state
        .db
        .get_config_snippet(app_type)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_claude_common_config_snippet(
    state: tauri::State<'_, crate::store::AppState>,
) -> Result<Option<String>, String> {
    get_common_config_snippet_internal(state.inner(), "claude")
}

#[tauri::command]
pub async fn set_claude_common_config_snippet(
    snippet: String,
    state: tauri::State<'_, crate::store::AppState>,
) -> Result<(), String> {
    let is_cleared = snippet.trim().is_empty();

    if !snippet.trim().is_empty() {
        serde_json::from_str::<serde_json::Value>(&snippet).map_err(invalid_json_format_error)?;
    }

    let value = if is_cleared { None } else { Some(snippet) };

    state
        .db
        .set_config_snippet("claude", value)
        .map_err(|e| e.to_string())?;
    state
        .db
        .set_config_snippet_cleared("claude", is_cleared)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_common_config_snippet(
    app_type: String,
    state: tauri::State<'_, crate::store::AppState>,
) -> Result<Option<String>, String> {
    get_common_config_snippet_internal(state.inner(), &app_type)
}

pub(crate) fn set_common_config_snippet_internal(
    state: &crate::store::AppState,
    app_type: &str,
    snippet: String,
) -> Result<(), ConfigCommandError> {
    let is_cleared = snippet.trim().is_empty();
    let old_snippet = state
        .db
        .get_config_snippet(app_type)
        .map_err(|e| ConfigCommandError::internal(e.to_string()))?;

    validate_common_config_snippet(app_type, &snippet).map_err(ConfigCommandError::bad_request)?;

    let value = if is_cleared { None } else { Some(snippet) };

    if matches!(app_type, "claude" | "codex" | "gemini") {
        if let Some(legacy_snippet) = old_snippet
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            let app = AppType::from_str(app_type)
                .map_err(|e| ConfigCommandError::bad_request(e.to_string()))?;
            crate::services::provider::ProviderService::migrate_legacy_common_config_usage(
                state,
                app,
                legacy_snippet,
            )
            .map_err(|e| ConfigCommandError::internal(e.to_string()))?;
        }
    }

    state
        .db
        .set_config_snippet(app_type, value)
        .map_err(|e| ConfigCommandError::internal(e.to_string()))?;
    state
        .db
        .set_config_snippet_cleared(app_type, is_cleared)
        .map_err(|e| ConfigCommandError::internal(e.to_string()))?;

    if matches!(app_type, "claude" | "codex" | "gemini") {
        let app = AppType::from_str(app_type)
            .map_err(|e| ConfigCommandError::bad_request(e.to_string()))?;
        crate::services::provider::ProviderService::sync_current_provider_for_app(
            state,
            app,
        )
        .map_err(|e| ConfigCommandError::internal(e.to_string()))?;
    }

    if app_type == "omo"
        && state
            .db
            .get_current_omo_provider("opencode", "omo")
            .map_err(|e| ConfigCommandError::internal(e.to_string()))?
            .is_some()
    {
        crate::services::OmoService::write_config_to_file(
            state,
            &crate::services::omo::STANDARD,
        )
        .map_err(|e| ConfigCommandError::internal(e.to_string()))?;
    }
    if app_type == "omo-slim"
        && state
            .db
            .get_current_omo_provider("opencode", "omo-slim")
            .map_err(|e| ConfigCommandError::internal(e.to_string()))?
            .is_some()
    {
        crate::services::OmoService::write_config_to_file(
            state,
            &crate::services::omo::SLIM,
        )
        .map_err(|e| ConfigCommandError::internal(e.to_string()))?;
    }
    Ok(())
}

#[tauri::command]
pub async fn set_common_config_snippet(
    app_type: String,
    snippet: String,
    state: tauri::State<'_, crate::store::AppState>,
) -> Result<(), String> {
    set_common_config_snippet_internal(state.inner(), &app_type, snippet)
        .map_err(ConfigCommandError::into_string)
}

#[cfg(test)]
mod tests {
    use super::validate_common_config_snippet;

    #[test]
    fn validate_common_config_snippet_accepts_comment_only_codex_snippet() {
        validate_common_config_snippet("codex", "# comment only\n")
            .expect("comment-only codex snippet should be valid");
    }

    #[test]
    fn validate_common_config_snippet_rejects_invalid_codex_snippet() {
        let err = validate_common_config_snippet("codex", "[broken")
            .expect_err("invalid codex snippet should be rejected");
        assert!(
            err.contains("TOML") || err.contains("toml") || err.contains("格式"),
            "expected TOML validation error, got {err}"
        );
    }
}

pub(crate) fn extract_common_config_snippet_internal(
    state: &crate::store::AppState,
    app: AppType,
    settings: Option<serde_json::Value>,
) -> Result<String, ConfigCommandError> {
    if let Some(settings) = settings {
        return crate::services::provider::ProviderService::extract_common_config_snippet_from_settings(
            app,
            &settings,
        )
        .map_err(|e| ConfigCommandError::internal(e.to_string()));
    }

    crate::services::provider::ProviderService::extract_common_config_snippet(state, app)
        .map_err(|e| ConfigCommandError::internal(e.to_string()))
}

#[tauri::command]
pub async fn extract_common_config_snippet(
    appType: String,
    settingsConfig: Option<String>,
    state: tauri::State<'_, crate::store::AppState>,
) -> Result<String, String> {
    let app = AppType::from_str(&appType).map_err(|e| e.to_string())?;

    let settings = if let Some(settings_config) = settingsConfig.filter(|s| !s.trim().is_empty()) {
        Some(serde_json::from_str(&settings_config).map_err(invalid_json_format_error)?)
    } else {
        None
    };

    extract_common_config_snippet_internal(state.inner(), app, settings)
        .map_err(ConfigCommandError::into_string)
}
