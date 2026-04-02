use indexmap::IndexMap;
use std::str::FromStr;

use crate::app_config::AppType;
use crate::prompt::Prompt;
use crate::services::PromptService;
use crate::store::AppState;

pub(crate) async fn get_prompts_internal(
    state: &AppState,
    app: String,
) -> Result<IndexMap<String, Prompt>, String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    PromptService::get_prompts(state, app_type).map_err(|e| e.to_string())
}

pub(crate) async fn upsert_prompt_internal(
    state: &AppState,
    app: String,
    id: String,
    prompt: Prompt,
) -> Result<(), String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    PromptService::upsert_prompt(state, app_type, &id, prompt).map_err(|e| e.to_string())
}

pub(crate) async fn delete_prompt_internal(
    state: &AppState,
    app: String,
    id: String,
) -> Result<(), String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    PromptService::delete_prompt(state, app_type, &id).map_err(|e| e.to_string())
}

pub(crate) async fn enable_prompt_internal(
    state: &AppState,
    app: String,
    id: String,
) -> Result<(), String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    PromptService::enable_prompt(state, app_type, &id).map_err(|e| e.to_string())
}

pub(crate) async fn import_prompt_from_file_internal(
    state: &AppState,
    app: String,
) -> Result<String, String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    PromptService::import_from_file(state, app_type).map_err(|e| e.to_string())
}

pub(crate) async fn get_current_prompt_file_content_internal(
    app: String,
) -> Result<Option<String>, String> {
    let app_type = AppType::from_str(&app).map_err(|e| e.to_string())?;
    PromptService::get_current_file_content(app_type).map_err(|e| e.to_string())
}
