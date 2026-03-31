use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use axum::extract::{DefaultBodyLimit, Multipart, Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, get_service, post, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

use crate::app_config::{AppType, McpServer};
use crate::database::FailoverQueueItem;
use crate::provider::Provider;
use crate::proxy::circuit_breaker::{CircuitBreakerConfig, CircuitBreakerStats};
use crate::proxy::types::{
    AppProxyConfig, GlobalProxyConfig, OptimizerConfig, ProviderHealth, ProxyConfig,
    ProxyServerInfo, ProxyStatus, ProxyTakeoverStatus, RectifierConfig,
};
use crate::prompt::Prompt;
use crate::services::skill::{
    DiscoverableSkill, ImportSkillSelection, SkillBackupEntry, SkillRepo, SkillUninstallResult,
};
use crate::services::{McpService, PromptService, ProviderService, SwitchResult};
use crate::store::AppState;
use crate::Database;

#[derive(Clone)]
struct WebApiState {
    app_state: Arc<AppState>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HealthResponse {
    status: &'static str,
    mode: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RootResponse {
    name: &'static str,
    mode: &'static str,
    api_base: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProvidersResponse {
    providers: indexmap::IndexMap<String, Provider>,
    current_provider_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorResponse {
    error: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EnabledRequest {
    enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ValueRequest {
    value: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderIdRequest {
    provider_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ToggleMcpAppRequest {
    enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ToggleSkillAppRequest {
    enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstallSkillRequest {
    skill: DiscoverableSkill,
    current_app: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurrentAppRequest {
    current_app: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContentRequest {
    content: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DailyMemorySearchQuery {
    query: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionMessagesQuery {
    provider_id: String,
    source_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageRangeQuery {
    start_date: Option<i64>,
    end_date: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestLogsPayload {
    filters: crate::services::usage_stats::LogFilters,
    page: Option<u32>,
    page_size: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateModelPricingPayload {
    display_name: String,
    input_cost: String,
    output_cost: String,
    cache_read_cost: String,
    cache_creation_cost: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UsageModelPricingInfo {
    model_id: String,
    display_name: String,
    input_cost_per_million: String,
    output_cost_per_million: String,
    cache_read_cost_per_million: String,
    cache_creation_cost_per_million: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SkillArchiveInstallResult {
    file_name: String,
    installed: Vec<crate::app_config::InstalledSkill>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

async fn get_mcp_servers(
    State(state): State<WebApiState>,
) -> Result<Json<indexmap::IndexMap<String, McpServer>>, ApiError> {
    let servers = McpService::get_all_servers(state.app_state.as_ref())
        .map_err(|e| ApiError::internal(format!("failed to load mcp servers: {e}")))?;
    Ok(Json(servers))
}

async fn upsert_mcp_server(
    State(state): State<WebApiState>,
    Json(server): Json<McpServer>,
) -> Result<StatusCode, ApiError> {
    McpService::upsert_server(state.app_state.as_ref(), server)
        .map_err(|e| ApiError::internal(format!("failed to save mcp server: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_mcp_server(
    State(state): State<WebApiState>,
    Path(id): Path<String>,
) -> Result<Json<bool>, ApiError> {
    let deleted = McpService::delete_server(state.app_state.as_ref(), &id)
        .map_err(|e| ApiError::internal(format!("failed to delete mcp server: {e}")))?;
    Ok(Json(deleted))
}

async fn toggle_mcp_app(
    State(state): State<WebApiState>,
    Path((id, app)): Path<(String, String)>,
    Json(payload): Json<ToggleMcpAppRequest>,
) -> Result<StatusCode, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    McpService::toggle_app(state.app_state.as_ref(), &id, app_type, payload.enabled)
        .map_err(|e| ApiError::internal(format!("failed to toggle mcp app: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn import_mcp_from_apps(State(state): State<WebApiState>) -> Result<Json<usize>, ApiError> {
    let mut total = 0;
    total += McpService::import_from_claude(state.app_state.as_ref()).unwrap_or(0);
    total += McpService::import_from_codex(state.app_state.as_ref()).unwrap_or(0);
    total += McpService::import_from_gemini(state.app_state.as_ref()).unwrap_or(0);
    total += McpService::import_from_opencode(state.app_state.as_ref()).unwrap_or(0);
    Ok(Json(total))
}

async fn get_prompts(
    State(state): State<WebApiState>,
    Path(app): Path<String>,
) -> Result<Json<indexmap::IndexMap<String, Prompt>>, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    let prompts = PromptService::get_prompts(state.app_state.as_ref(), app_type)
        .map_err(|e| ApiError::internal(format!("failed to load prompts: {e}")))?;
    Ok(Json(prompts))
}

async fn upsert_prompt(
    State(state): State<WebApiState>,
    Path((app, id)): Path<(String, String)>,
    Json(prompt): Json<Prompt>,
) -> Result<StatusCode, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    PromptService::upsert_prompt(state.app_state.as_ref(), app_type, &id, prompt)
        .map_err(|e| ApiError::internal(format!("failed to save prompt: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_prompt(
    State(state): State<WebApiState>,
    Path((app, id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    PromptService::delete_prompt(state.app_state.as_ref(), app_type, &id)
        .map_err(|e| ApiError::internal(format!("failed to delete prompt: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn enable_prompt(
    State(state): State<WebApiState>,
    Path((app, id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    PromptService::enable_prompt(state.app_state.as_ref(), app_type, &id)
        .map_err(|e| ApiError::internal(format!("failed to enable prompt: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn import_prompt_from_file(
    State(state): State<WebApiState>,
    Path(app): Path<String>,
) -> Result<Json<String>, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    let id = PromptService::import_from_file(state.app_state.as_ref(), app_type)
        .map_err(|e| ApiError::internal(format!("failed to import prompt from file: {e}")))?;
    Ok(Json(id))
}

async fn get_current_prompt_file_content(
    Path(app): Path<String>,
) -> Result<Json<Option<String>>, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    let content = PromptService::get_current_file_content(app_type)
        .map_err(|e| ApiError::internal(format!("failed to load current prompt file: {e}")))?;
    Ok(Json(content))
}

async fn get_live_provider_ids(
    Path(app): Path<String>,
) -> Result<Json<Vec<String>>, ApiError> {
    let provider_ids = match AppType::from_str(&app) {
        Ok(AppType::OpenCode) => crate::opencode_config::get_providers()
            .map(|providers| providers.keys().cloned().collect()),
        Ok(AppType::OpenClaw) => crate::openclaw_config::get_providers()
            .map(|providers| providers.keys().cloned().collect()),
        Ok(app_type) => {
            return Err(ApiError::bad_request(format!(
                "{} does not support live provider ids",
                app_type.as_str()
            )));
        }
        Err(err) => return Err(ApiError::bad_request(err.to_string())),
    }
    .map_err(|e| ApiError::internal(format!("failed to load live provider ids: {e}")))?;

    Ok(Json(provider_ids))
}

async fn import_providers_from_live(
    State(state): State<WebApiState>,
    Path(app): Path<String>,
) -> Result<Json<usize>, ApiError> {
    let imported = match AppType::from_str(&app) {
        Ok(AppType::OpenCode) => {
            crate::services::provider::import_opencode_providers_from_live(state.app_state.as_ref())
        }
        Ok(AppType::OpenClaw) => {
            crate::services::provider::import_openclaw_providers_from_live(state.app_state.as_ref())
        }
        Ok(app_type) => {
            return Err(ApiError::bad_request(format!(
                "{} does not support importing providers from live config",
                app_type.as_str()
            )));
        }
        Err(err) => return Err(ApiError::bad_request(err.to_string())),
    }
    .map_err(|e| ApiError::internal(format!("failed to import live providers: {e}")))?;

    Ok(Json(imported))
}

async fn remove_provider_from_live_config(
    State(state): State<WebApiState>,
    Path((app, id)): Path<(String, String)>,
) -> Result<Json<bool>, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    crate::services::ProviderService::remove_from_live_config(state.app_state.as_ref(), app_type, &id)
        .map_err(|e| ApiError::internal(format!("failed to remove provider from live config: {e}")))?;
    Ok(Json(true))
}

async fn get_installed_skills(
    State(state): State<WebApiState>,
) -> Result<Json<Vec<crate::app_config::InstalledSkill>>, ApiError> {
    let skills = crate::services::skill::SkillService::get_all_installed(&state.app_state.db)
        .map_err(|e| ApiError::internal(format!("failed to load installed skills: {e}")))?;
    Ok(Json(skills))
}

async fn get_skill_backups() -> Result<Json<Vec<SkillBackupEntry>>, ApiError> {
    let backups = crate::services::skill::SkillService::list_backups()
        .map_err(|e| ApiError::internal(format!("failed to load skill backups: {e}")))?;
    Ok(Json(backups))
}

async fn uninstall_skill_unified(
    State(state): State<WebApiState>,
    Path(id): Path<String>,
) -> Result<Json<SkillUninstallResult>, ApiError> {
    let result = crate::services::skill::SkillService::uninstall(&state.app_state.db, &id)
        .map_err(|e| ApiError::internal(format!("failed to uninstall skill: {e}")))?;
    Ok(Json(result))
}

async fn toggle_skill_app(
    State(state): State<WebApiState>,
    Path((id, app)): Path<(String, String)>,
    Json(payload): Json<ToggleSkillAppRequest>,
) -> Result<Json<bool>, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    crate::services::skill::SkillService::toggle_app(
        &state.app_state.db,
        &id,
        &app_type,
        payload.enabled,
    )
    .map_err(|e| ApiError::internal(format!("failed to toggle skill app: {e}")))?;
    Ok(Json(true))
}

async fn scan_unmanaged_skills(
    State(state): State<WebApiState>,
) -> Result<Json<Vec<crate::app_config::UnmanagedSkill>>, ApiError> {
    let skills = crate::services::skill::SkillService::scan_unmanaged(&state.app_state.db)
        .map_err(|e| ApiError::internal(format!("failed to scan unmanaged skills: {e}")))?;
    Ok(Json(skills))
}

async fn import_skills_from_apps(
    State(state): State<WebApiState>,
    Json(imports): Json<Vec<ImportSkillSelection>>,
) -> Result<Json<Vec<crate::app_config::InstalledSkill>>, ApiError> {
    let skills = crate::services::skill::SkillService::import_from_apps(&state.app_state.db, imports)
        .map_err(|e| ApiError::internal(format!("failed to import skills from apps: {e}")))?;
    Ok(Json(skills))
}

async fn get_skill_repos(
    State(state): State<WebApiState>,
) -> Result<Json<Vec<SkillRepo>>, ApiError> {
    let repos = state
        .app_state
        .db
        .get_skill_repos()
        .map_err(|e| ApiError::internal(format!("failed to load skill repos: {e}")))?;
    Ok(Json(repos))
}

async fn add_skill_repo(
    State(state): State<WebApiState>,
    Json(repo): Json<SkillRepo>,
) -> Result<Json<bool>, ApiError> {
    state
        .app_state
        .db
        .save_skill_repo(&repo)
        .map_err(|e| ApiError::internal(format!("failed to save skill repo: {e}")))?;
    Ok(Json(true))
}

async fn remove_skill_repo(
    State(state): State<WebApiState>,
    Path((owner, name)): Path<(String, String)>,
) -> Result<Json<bool>, ApiError> {
    state
        .app_state
        .db
        .delete_skill_repo(&owner, &name)
        .map_err(|e| ApiError::internal(format!("failed to remove skill repo: {e}")))?;
    Ok(Json(true))
}

async fn discover_available_skills(
    State(state): State<WebApiState>,
) -> Result<Json<Vec<DiscoverableSkill>>, ApiError> {
    let repos = state
        .app_state
        .db
        .get_skill_repos()
        .map_err(|e| ApiError::internal(format!("failed to load skill repos: {e}")))?;
    let skills = crate::services::skill::SkillService::new()
        .discover_available(repos)
        .await
        .map_err(|e| ApiError::internal(format!("failed to discover skills: {e}")))?;
    Ok(Json(skills))
}

async fn install_skill_unified(
    State(state): State<WebApiState>,
    Json(payload): Json<InstallSkillRequest>,
) -> Result<Json<crate::app_config::InstalledSkill>, ApiError> {
    let app_type = AppType::from_str(&payload.current_app)
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    let installed = crate::services::skill::SkillService::new()
        .install(&state.app_state.db, &payload.skill, &app_type)
        .await
        .map_err(|e| ApiError::internal(format!("failed to install skill: {e}")))?;
    Ok(Json(installed))
}

async fn delete_skill_backup(
    Path(backup_id): Path<String>,
) -> Result<Json<bool>, ApiError> {
    crate::services::skill::SkillService::delete_backup(&backup_id)
        .map_err(|e| ApiError::internal(format!("failed to delete skill backup: {e}")))?;
    Ok(Json(true))
}

async fn restore_skill_backup(
    State(state): State<WebApiState>,
    Path(backup_id): Path<String>,
    Json(payload): Json<CurrentAppRequest>,
) -> Result<Json<crate::app_config::InstalledSkill>, ApiError> {
    let app_type = AppType::from_str(&payload.current_app)
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    let restored = crate::services::skill::SkillService::restore_from_backup(
        &state.app_state.db,
        &backup_id,
        &app_type,
    )
    .map_err(|e| ApiError::internal(format!("failed to restore skill backup: {e}")))?;
    Ok(Json(restored))
}

fn resolve_workspace_directory_path(subdir: &str) -> PathBuf {
    match subdir {
        "memory" => crate::openclaw_config::get_openclaw_dir()
            .join("workspace")
            .join("memory"),
        _ => crate::openclaw_config::get_openclaw_dir().join("workspace"),
    }
}

async fn get_workspace_file(
    Path(filename): Path<String>,
) -> Result<Json<Option<String>>, ApiError> {
    let content = crate::read_workspace_file(filename)
        .await
        .map_err(|e| ApiError::internal(format!("failed to read workspace file: {e}")))?;
    Ok(Json(content))
}

async fn save_workspace_file(
    Path(filename): Path<String>,
    Json(payload): Json<ContentRequest>,
) -> Result<StatusCode, ApiError> {
    crate::write_workspace_file(filename, payload.content)
        .await
        .map_err(|e| ApiError::internal(format!("failed to write workspace file: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_workspace_daily_memory_files(
) -> Result<Json<Vec<crate::DailyMemoryFileInfo>>, ApiError> {
    let files = crate::list_daily_memory_files()
        .await
        .map_err(|e| ApiError::internal(format!("failed to list daily memory files: {e}")))?;
    Ok(Json(files))
}

async fn search_workspace_daily_memory_files(
    Query(query): Query<DailyMemorySearchQuery>,
) -> Result<Json<Vec<crate::DailyMemorySearchResult>>, ApiError> {
    let results = crate::search_daily_memory_files(query.query)
        .await
        .map_err(|e| ApiError::internal(format!("failed to search daily memory files: {e}")))?;
    Ok(Json(results))
}

async fn get_workspace_daily_memory_file(
    Path(filename): Path<String>,
) -> Result<Json<Option<String>>, ApiError> {
    let content = crate::read_daily_memory_file(filename)
        .await
        .map_err(|e| ApiError::internal(format!("failed to read daily memory file: {e}")))?;
    Ok(Json(content))
}

async fn save_workspace_daily_memory_file(
    Path(filename): Path<String>,
    Json(payload): Json<ContentRequest>,
) -> Result<StatusCode, ApiError> {
    crate::write_daily_memory_file(filename, payload.content)
        .await
        .map_err(|e| ApiError::internal(format!("failed to write daily memory file: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_workspace_daily_memory_file(
    Path(filename): Path<String>,
) -> Result<StatusCode, ApiError> {
    crate::delete_daily_memory_file(filename)
        .await
        .map_err(|e| ApiError::internal(format!("failed to delete daily memory file: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_workspace_directory_path(
    Path(subdir): Path<String>,
) -> Result<Json<String>, ApiError> {
    let path = resolve_workspace_directory_path(&subdir);
    std::fs::create_dir_all(&path)
        .map_err(|e| ApiError::internal(format!("failed to prepare workspace directory: {e}")))?;
    Ok(Json(path.to_string_lossy().to_string()))
}

async fn get_openclaw_default_model(
) -> Result<Json<Option<crate::openclaw_config::OpenClawDefaultModel>>, ApiError> {
    let model = crate::openclaw_config::get_default_model()
        .map_err(|e| ApiError::internal(format!("failed to load openclaw default model: {e}")))?;
    Ok(Json(model))
}

async fn set_openclaw_default_model(
    Json(model): Json<crate::openclaw_config::OpenClawDefaultModel>,
) -> Result<Json<crate::openclaw_config::OpenClawWriteOutcome>, ApiError> {
    let outcome = crate::openclaw_config::set_default_model(&model)
        .map_err(|e| ApiError::internal(format!("failed to save openclaw default model: {e}")))?;
    Ok(Json(outcome))
}

async fn get_openclaw_model_catalog(
) -> Result<Json<Option<HashMap<String, crate::openclaw_config::OpenClawModelCatalogEntry>>>, ApiError>
{
    let catalog = crate::openclaw_config::get_model_catalog()
        .map_err(|e| ApiError::internal(format!("failed to load openclaw model catalog: {e}")))?;
    Ok(Json(catalog))
}

async fn set_openclaw_model_catalog(
    Json(catalog): Json<HashMap<String, crate::openclaw_config::OpenClawModelCatalogEntry>>,
) -> Result<Json<crate::openclaw_config::OpenClawWriteOutcome>, ApiError> {
    let outcome = crate::openclaw_config::set_model_catalog(&catalog)
        .map_err(|e| ApiError::internal(format!("failed to save openclaw model catalog: {e}")))?;
    Ok(Json(outcome))
}

async fn get_openclaw_agents_defaults(
) -> Result<Json<Option<crate::openclaw_config::OpenClawAgentsDefaults>>, ApiError> {
    let defaults = crate::openclaw_config::get_agents_defaults()
        .map_err(|e| ApiError::internal(format!("failed to load openclaw agents defaults: {e}")))?;
    Ok(Json(defaults))
}

async fn set_openclaw_agents_defaults(
    Json(defaults): Json<crate::openclaw_config::OpenClawAgentsDefaults>,
) -> Result<Json<crate::openclaw_config::OpenClawWriteOutcome>, ApiError> {
    let outcome = crate::openclaw_config::set_agents_defaults(&defaults).map_err(|e| {
        ApiError::internal(format!("failed to save openclaw agents defaults: {e}"))
    })?;
    Ok(Json(outcome))
}

async fn get_openclaw_env(
) -> Result<Json<crate::openclaw_config::OpenClawEnvConfig>, ApiError> {
    let env = crate::openclaw_config::get_env_config()
        .map_err(|e| ApiError::internal(format!("failed to load openclaw env config: {e}")))?;
    Ok(Json(env))
}

async fn set_openclaw_env(
    Json(env): Json<crate::openclaw_config::OpenClawEnvConfig>,
) -> Result<Json<crate::openclaw_config::OpenClawWriteOutcome>, ApiError> {
    let outcome = crate::openclaw_config::set_env_config(&env)
        .map_err(|e| ApiError::internal(format!("failed to save openclaw env config: {e}")))?;
    Ok(Json(outcome))
}

async fn get_openclaw_tools(
) -> Result<Json<crate::openclaw_config::OpenClawToolsConfig>, ApiError> {
    let tools = crate::openclaw_config::get_tools_config()
        .map_err(|e| ApiError::internal(format!("failed to load openclaw tools config: {e}")))?;
    Ok(Json(tools))
}

async fn set_openclaw_tools(
    Json(tools): Json<crate::openclaw_config::OpenClawToolsConfig>,
) -> Result<Json<crate::openclaw_config::OpenClawWriteOutcome>, ApiError> {
    let outcome = crate::openclaw_config::set_tools_config(&tools)
        .map_err(|e| ApiError::internal(format!("failed to save openclaw tools config: {e}")))?;
    Ok(Json(outcome))
}

async fn scan_openclaw_config_health(
) -> Result<Json<Vec<crate::openclaw_config::OpenClawHealthWarning>>, ApiError> {
    let warnings = crate::openclaw_config::scan_openclaw_config_health()
        .map_err(|e| ApiError::internal(format!("failed to scan openclaw config health: {e}")))?;
    Ok(Json(warnings))
}

async fn get_openclaw_live_provider(
    Path(provider_id): Path<String>,
) -> Result<Json<Option<serde_json::Value>>, ApiError> {
    let provider = crate::openclaw_config::get_provider(&provider_id)
        .map_err(|e| ApiError::internal(format!("failed to load openclaw live provider: {e}")))?;
    Ok(Json(provider))
}

async fn list_sessions() -> Result<Json<Vec<crate::session_manager::SessionMeta>>, ApiError> {
    let sessions = crate::list_sessions()
        .await
        .map_err(|e| ApiError::internal(format!("failed to list sessions: {e}")))?;
    Ok(Json(sessions))
}

async fn get_session_messages(
    Query(query): Query<SessionMessagesQuery>,
) -> Result<Json<Vec<crate::session_manager::SessionMessage>>, ApiError> {
    let messages = crate::get_session_messages(query.provider_id, query.source_path)
        .await
        .map_err(|e| ApiError::internal(format!("failed to load session messages: {e}")))?;
    Ok(Json(messages))
}

async fn delete_session(
    Json(payload): Json<crate::session_manager::DeleteSessionRequest>,
) -> Result<Json<bool>, ApiError> {
    let deleted = crate::delete_session(payload.provider_id, payload.session_id, payload.source_path)
        .await
        .map_err(|e| ApiError::internal(format!("failed to delete session: {e}")))?;
    Ok(Json(deleted))
}

async fn delete_sessions(
    Json(items): Json<Vec<crate::session_manager::DeleteSessionRequest>>,
) -> Result<Json<Vec<crate::session_manager::DeleteSessionOutcome>>, ApiError> {
    let results = crate::delete_sessions(items)
        .await
        .map_err(|e| ApiError::internal(format!("failed to delete sessions: {e}")))?;
    Ok(Json(results))
}

async fn get_usage_summary(
    State(state): State<WebApiState>,
    Query(query): Query<UsageRangeQuery>,
) -> Result<Json<crate::services::usage_stats::UsageSummary>, ApiError> {
    let summary = state
        .app_state
        .db
        .get_usage_summary(query.start_date, query.end_date)
        .map_err(|e| ApiError::internal(format!("failed to load usage summary: {e}")))?;
    Ok(Json(summary))
}

async fn get_usage_trends(
    State(state): State<WebApiState>,
    Query(query): Query<UsageRangeQuery>,
) -> Result<Json<Vec<crate::services::usage_stats::DailyStats>>, ApiError> {
    let trends = state
        .app_state
        .db
        .get_daily_trends(query.start_date, query.end_date)
        .map_err(|e| ApiError::internal(format!("failed to load usage trends: {e}")))?;
    Ok(Json(trends))
}

async fn get_usage_provider_stats(
    State(state): State<WebApiState>,
) -> Result<Json<Vec<crate::services::usage_stats::ProviderStats>>, ApiError> {
    let stats = state
        .app_state
        .db
        .get_provider_stats()
        .map_err(|e| ApiError::internal(format!("failed to load provider stats: {e}")))?;
    Ok(Json(stats))
}

async fn get_usage_model_stats(
    State(state): State<WebApiState>,
) -> Result<Json<Vec<crate::services::usage_stats::ModelStats>>, ApiError> {
    let stats = state
        .app_state
        .db
        .get_model_stats()
        .map_err(|e| ApiError::internal(format!("failed to load model stats: {e}")))?;
    Ok(Json(stats))
}

async fn get_usage_request_logs(
    State(state): State<WebApiState>,
    Json(payload): Json<RequestLogsPayload>,
) -> Result<Json<crate::services::usage_stats::PaginatedLogs>, ApiError> {
    let logs = state
        .app_state
        .db
        .get_request_logs(
            &payload.filters,
            payload.page.unwrap_or(0),
            payload.page_size.unwrap_or(20),
        )
        .map_err(|e| ApiError::internal(format!("failed to load request logs: {e}")))?;
    Ok(Json(logs))
}

async fn get_usage_request_detail(
    State(state): State<WebApiState>,
    Path(request_id): Path<String>,
) -> Result<Json<Option<crate::services::usage_stats::RequestLogDetail>>, ApiError> {
    let detail = state
        .app_state
        .db
        .get_request_detail(&request_id)
        .map_err(|e| ApiError::internal(format!("failed to load request detail: {e}")))?;
    Ok(Json(detail))
}

async fn get_usage_model_pricing(
    State(state): State<WebApiState>,
) -> Result<Json<Vec<UsageModelPricingInfo>>, ApiError> {
    state
        .app_state
        .db
        .ensure_model_pricing_seeded()
        .map_err(|e| ApiError::internal(format!("failed to seed model pricing: {e}")))?;

    let db = state.app_state.db.clone();
    let conn = db
        .conn
        .lock()
        .map_err(|e| ApiError::internal(format!("failed to lock database connection: {e}")))?;

    let mut stmt = conn
        .prepare(
            "SELECT model_id, display_name, input_cost_per_million, output_cost_per_million,
                    cache_read_cost_per_million, cache_creation_cost_per_million
             FROM model_pricing
             ORDER BY display_name",
        )
        .map_err(|e| ApiError::internal(format!("failed to prepare pricing query: {e}")))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(UsageModelPricingInfo {
                model_id: row.get(0)?,
                display_name: row.get(1)?,
                input_cost_per_million: row.get(2)?,
                output_cost_per_million: row.get(3)?,
                cache_read_cost_per_million: row.get(4)?,
                cache_creation_cost_per_million: row.get(5)?,
            })
        })
        .map_err(|e| ApiError::internal(format!("failed to query pricing rows: {e}")))?;

    let mut pricing = Vec::new();
    for row in rows {
        pricing.push(
            row.map_err(|e| ApiError::internal(format!("failed to decode pricing row: {e}")))?,
        );
    }

    Ok(Json(pricing))
}

async fn update_usage_model_pricing(
    State(state): State<WebApiState>,
    Path(model_id): Path<String>,
    Json(payload): Json<UpdateModelPricingPayload>,
) -> Result<StatusCode, ApiError> {
    let db = state.app_state.db.clone();
    let conn = db
        .conn
        .lock()
        .map_err(|e| ApiError::internal(format!("failed to lock database connection: {e}")))?;
    conn.execute(
        "INSERT OR REPLACE INTO model_pricing (
            model_id, display_name, input_cost_per_million, output_cost_per_million,
            cache_read_cost_per_million, cache_creation_cost_per_million
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            model_id,
            payload.display_name,
            payload.input_cost,
            payload.output_cost,
            payload.cache_read_cost,
            payload.cache_creation_cost
        ],
    )
    .map_err(|e| ApiError::internal(format!("failed to update model pricing: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_usage_model_pricing(
    State(state): State<WebApiState>,
    Path(model_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let db = state.app_state.db.clone();
    let conn = db
        .conn
        .lock()
        .map_err(|e| ApiError::internal(format!("failed to lock database connection: {e}")))?;
    conn.execute(
        "DELETE FROM model_pricing WHERE model_id = ?1",
        rusqlite::params![model_id],
    )
    .map_err(|e| ApiError::internal(format!("failed to delete model pricing: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_usage_provider_limits(
    State(state): State<WebApiState>,
    Path((app_type, provider_id)): Path<(String, String)>,
) -> Result<Json<crate::services::usage_stats::ProviderLimitStatus>, ApiError> {
    let limits = state
        .app_state
        .db
        .check_provider_limits(&provider_id, &app_type)
        .map_err(|e| ApiError::internal(format!("failed to load provider limits: {e}")))?;
    Ok(Json(limits))
}

fn sanitize_uploaded_archive_name(file_name: &str, index: usize) -> String {
    let fallback = format!("skill-archive-{}.zip", index + 1);
    let candidate = std::path::Path::new(file_name)
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or(&fallback);

    let mut sanitized = candidate
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => ch,
        })
        .collect::<String>();

    if sanitized.trim().is_empty() {
        sanitized = fallback;
    }

    if !sanitized.to_ascii_lowercase().ends_with(".zip") {
        sanitized.push_str(".zip");
    }

    sanitized
}

async fn install_skill_archives(
    State(state): State<WebApiState>,
    mut multipart: Multipart,
) -> Result<Json<Vec<SkillArchiveInstallResult>>, ApiError> {
    let mut current_app: Option<String> = None;
    let mut uploads: Vec<(String, bytes::Bytes)> = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::bad_request(format!("failed to read upload field: {e}")))?
    {
        match field.name() {
            Some("currentApp") => {
                current_app = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| {
                            ApiError::bad_request(format!(
                                "failed to read currentApp from upload payload: {e}"
                            ))
                        })?
                        .trim()
                        .to_string(),
                );
            }
            Some("archives") => {
                let file_name = field.file_name().unwrap_or("skill-archive.zip").to_string();
                let bytes = field.bytes().await.map_err(|e| {
                    ApiError::bad_request(format!(
                        "failed to read uploaded archive {file_name}: {e}"
                    ))
                })?;
                uploads.push((file_name, bytes));
            }
            _ => {}
        }
    }

    let current_app = current_app
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ApiError::bad_request("missing currentApp in upload payload"))?;
    let app_type = AppType::from_str(&current_app).map_err(|e| ApiError::bad_request(e.to_string()))?;

    if uploads.is_empty() {
        return Err(ApiError::bad_request("missing archives in upload payload"));
    }

    let temp_dir = tempfile::tempdir()
        .map_err(|e| ApiError::internal(format!("failed to prepare upload temp dir: {e}")))?;
    let mut results = Vec::with_capacity(uploads.len());

    for (index, (original_name, bytes)) in uploads.into_iter().enumerate() {
        let is_zip = std::path::Path::new(&original_name)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("zip"))
            .unwrap_or(false);
        let file_name = sanitize_uploaded_archive_name(&original_name, index);
        if !is_zip {
            results.push(SkillArchiveInstallResult {
                file_name,
                installed: Vec::new(),
                error: Some("only .zip archives are supported".to_string()),
            });
            continue;
        }

        let archive_path = temp_dir.path().join(&file_name);
        std::fs::write(&archive_path, &bytes).map_err(|e| {
            ApiError::internal(format!(
                "failed to persist uploaded archive {}: {e}",
                archive_path.display()
            ))
        })?;

        let install_result =
            crate::services::skill::SkillService::install_from_zip(&state.app_state.db, &archive_path, &app_type);

        let _ = std::fs::remove_file(&archive_path);

        match install_result {
            Ok(installed) => results.push(SkillArchiveInstallResult {
                file_name,
                installed,
                error: None,
            }),
            Err(error) => results.push(SkillArchiveInstallResult {
                file_name,
                installed: Vec::new(),
                error: Some(error.to_string()),
            }),
        }
    }

    Ok(Json(results))
}

fn merge_settings_for_save(
    mut incoming: crate::settings::AppSettings,
    existing: &crate::settings::AppSettings,
) -> crate::settings::AppSettings {
    match (&mut incoming.webdav_sync, &existing.webdav_sync) {
        (None, _) => {
            incoming.webdav_sync = existing.webdav_sync.clone();
        }
        (Some(incoming_sync), Some(existing_sync))
            if incoming_sync.password.is_empty() && !existing_sync.password.is_empty() =>
        {
            incoming_sync.password = existing_sync.password.clone();
        }
        _ => {}
    }

    incoming
}

struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(ErrorResponse { error: self.message })).into_response()
    }
}

async fn root() -> Json<RootResponse> {
    Json(RootResponse {
        name: "cc-switch-web",
        mode: "local-rust-service",
        api_base: "/api",
    })
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        mode: "local-rust-service",
    })
}

async fn get_settings() -> Json<crate::settings::AppSettings> {
    Json(crate::settings::get_settings_for_frontend())
}

async fn save_settings(
    Json(settings): Json<crate::settings::AppSettings>,
) -> Result<Json<bool>, ApiError> {
    let existing = crate::settings::get_settings();
    let merged = merge_settings_for_save(settings, &existing);
    crate::settings::update_settings(merged)
        .map_err(|e| ApiError::internal(format!("failed to save settings: {e}")))?;
    Ok(Json(true))
}

async fn get_rectifier_config(
    State(state): State<WebApiState>,
) -> Result<Json<RectifierConfig>, ApiError> {
    let config = state
        .app_state
        .db
        .get_rectifier_config()
        .map_err(|e| ApiError::internal(format!("failed to load rectifier config: {e}")))?;
    Ok(Json(config))
}

async fn set_rectifier_config(
    State(state): State<WebApiState>,
    Json(config): Json<RectifierConfig>,
) -> Result<Json<bool>, ApiError> {
    state
        .app_state
        .db
        .set_rectifier_config(&config)
        .map_err(|e| ApiError::internal(format!("failed to save rectifier config: {e}")))?;
    Ok(Json(true))
}

async fn get_optimizer_config(
    State(state): State<WebApiState>,
) -> Result<Json<OptimizerConfig>, ApiError> {
    let config = state
        .app_state
        .db
        .get_optimizer_config()
        .map_err(|e| ApiError::internal(format!("failed to load optimizer config: {e}")))?;
    Ok(Json(config))
}

async fn set_optimizer_config(
    State(state): State<WebApiState>,
    Json(config): Json<OptimizerConfig>,
) -> Result<Json<bool>, ApiError> {
    match config.cache_ttl.as_str() {
        "5m" | "1h" => {}
        other => {
            return Err(ApiError::bad_request(format!(
                "Invalid cache_ttl value: '{other}'. Allowed values: '5m', '1h'"
            )));
        }
    }

    state
        .app_state
        .db
        .set_optimizer_config(&config)
        .map_err(|e| ApiError::internal(format!("failed to save optimizer config: {e}")))?;
    Ok(Json(true))
}

async fn get_providers(
    State(state): State<WebApiState>,
    Path(app): Path<String>,
) -> Result<Json<ProvidersResponse>, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    let providers =
        ProviderService::list(state.app_state.as_ref(), app_type.clone()).map_err(|e| {
            ApiError::internal(format!("failed to load providers for {}: {e}", app_type.as_str()))
        })?;
    let current_provider_id =
        ProviderService::current(state.app_state.as_ref(), app_type.clone()).map_err(|e| {
            ApiError::internal(format!(
                "failed to load current provider for {}: {e}",
                app_type.as_str()
            ))
        })?;

    Ok(Json(ProvidersResponse {
        providers,
        current_provider_id,
    }))
}

async fn get_current_provider(
    State(state): State<WebApiState>,
    Path(app): Path<String>,
) -> Result<Json<String>, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    let current_provider_id =
        ProviderService::current(state.app_state.as_ref(), app_type.clone()).map_err(|e| {
            ApiError::internal(format!(
                "failed to load current provider for {}: {e}",
                app_type.as_str()
            ))
        })?;

    Ok(Json(current_provider_id))
}

async fn add_provider(
    State(state): State<WebApiState>,
    Path(app): Path<String>,
    Json(provider): Json<Provider>,
) -> Result<Json<bool>, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    let added = ProviderService::add(state.app_state.as_ref(), app_type, provider)
        .map_err(|e| ApiError::internal(format!("failed to add provider: {e}")))?;
    Ok(Json(added))
}

async fn update_provider(
    State(state): State<WebApiState>,
    Path((app, id)): Path<(String, String)>,
    Json(mut provider): Json<Provider>,
) -> Result<Json<bool>, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    provider.id = id;
    let updated = ProviderService::update(state.app_state.as_ref(), app_type, provider)
        .map_err(|e| ApiError::internal(format!("failed to update provider: {e}")))?;
    Ok(Json(updated))
}

async fn delete_provider(
    State(state): State<WebApiState>,
    Path((app, id)): Path<(String, String)>,
) -> Result<Json<bool>, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    ProviderService::delete(state.app_state.as_ref(), app_type, &id)
        .map_err(|e| ApiError::internal(format!("failed to delete provider: {e}")))?;
    Ok(Json(true))
}

async fn switch_provider(
    State(state): State<WebApiState>,
    Path((app, id)): Path<(String, String)>,
) -> Result<Json<SwitchResult>, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    let result = ProviderService::switch(state.app_state.as_ref(), app_type, &id)
        .map_err(|e| ApiError::internal(format!("failed to switch provider: {e}")))?;
    Ok(Json(result))
}

async fn get_proxy_status(State(state): State<WebApiState>) -> Result<Json<ProxyStatus>, ApiError> {
    let status = state
        .app_state
        .proxy_service
        .get_status()
        .await
        .map_err(|e| ApiError::internal(format!("failed to load proxy status: {e}")))?;
    Ok(Json(status))
}

async fn get_proxy_takeover_status(
    State(state): State<WebApiState>,
) -> Result<Json<ProxyTakeoverStatus>, ApiError> {
    let status = state
        .app_state
        .proxy_service
        .get_takeover_status()
        .await
        .map_err(|e| ApiError::internal(format!("failed to load proxy takeover status: {e}")))?;
    Ok(Json(status))
}

async fn get_proxy_config(State(state): State<WebApiState>) -> Result<Json<ProxyConfig>, ApiError> {
    let config = state
        .app_state
        .proxy_service
        .get_config()
        .await
        .map_err(|e| ApiError::internal(format!("failed to load proxy config: {e}")))?;
    Ok(Json(config))
}

async fn get_global_proxy_config(
    State(state): State<WebApiState>,
) -> Result<Json<GlobalProxyConfig>, ApiError> {
    let config = state
        .app_state
        .db
        .get_global_proxy_config()
        .await
        .map_err(|e| ApiError::internal(format!("failed to load global proxy config: {e}")))?;
    Ok(Json(config))
}

async fn get_proxy_config_for_app(
    State(state): State<WebApiState>,
    Path(app): Path<String>,
) -> Result<Json<AppProxyConfig>, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    let config = state
        .app_state
        .db
        .get_proxy_config_for_app(app_type.as_str())
        .await
        .map_err(|e| ApiError::internal(format!("failed to load app proxy config: {e}")))?;
    Ok(Json(config))
}

async fn is_proxy_running(State(state): State<WebApiState>) -> Json<bool> {
    Json(state.app_state.proxy_service.is_running().await)
}

async fn is_live_takeover_active(
    State(state): State<WebApiState>,
) -> Result<Json<bool>, ApiError> {
    let active = state
        .app_state
        .proxy_service
        .is_takeover_active()
        .await
        .map_err(|e| ApiError::internal(format!("failed to load proxy takeover state: {e}")))?;
    Ok(Json(active))
}

async fn start_proxy_server(
    State(state): State<WebApiState>,
) -> Result<Json<ProxyServerInfo>, ApiError> {
    let info = state
        .app_state
        .proxy_service
        .start()
        .await
        .map_err(|e| ApiError::internal(format!("failed to start proxy server: {e}")))?;
    Ok(Json(info))
}

async fn stop_proxy_with_restore(State(state): State<WebApiState>) -> Result<StatusCode, ApiError> {
    state
        .app_state
        .proxy_service
        .stop_with_restore()
        .await
        .map_err(|e| ApiError::internal(format!("failed to stop proxy server: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn update_proxy_config(
    State(state): State<WebApiState>,
    Json(config): Json<ProxyConfig>,
) -> Result<StatusCode, ApiError> {
    state
        .app_state
        .proxy_service
        .update_config(&config)
        .await
        .map_err(|e| ApiError::internal(format!("failed to update proxy config: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn update_global_proxy_config(
    State(state): State<WebApiState>,
    Json(config): Json<GlobalProxyConfig>,
) -> Result<StatusCode, ApiError> {
    state
        .app_state
        .db
        .update_global_proxy_config(config)
        .await
        .map_err(|e| ApiError::internal(format!("failed to update global proxy config: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn update_proxy_config_for_app(
    State(state): State<WebApiState>,
    Path(app): Path<String>,
    Json(mut config): Json<AppProxyConfig>,
) -> Result<StatusCode, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    config.app_type = app_type.as_str().to_string();
    state
        .app_state
        .db
        .update_proxy_config_for_app(config)
        .await
        .map_err(|e| ApiError::internal(format!("failed to update app proxy config: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn set_proxy_takeover_for_app(
    State(state): State<WebApiState>,
    Path(app): Path<String>,
    Json(payload): Json<EnabledRequest>,
) -> Result<StatusCode, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    state
        .app_state
        .proxy_service
        .set_takeover_for_app(app_type.as_str(), payload.enabled)
        .await
        .map_err(|e| ApiError::internal(format!("failed to update proxy takeover: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn switch_proxy_provider(
    State(state): State<WebApiState>,
    Path((app, id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    state
        .app_state
        .proxy_service
        .switch_proxy_target(app_type.as_str(), &id)
        .await
        .map_err(|e| ApiError::internal(format!("failed to switch proxy provider: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_default_cost_multiplier(
    State(state): State<WebApiState>,
    Path(app): Path<String>,
) -> Result<Json<String>, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    let value = state
        .app_state
        .db
        .get_default_cost_multiplier(app_type.as_str())
        .await
        .map_err(|e| ApiError::internal(format!("failed to load default cost multiplier: {e}")))?;
    Ok(Json(value))
}

async fn set_default_cost_multiplier(
    State(state): State<WebApiState>,
    Path(app): Path<String>,
    Json(payload): Json<ValueRequest>,
) -> Result<StatusCode, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    state
        .app_state
        .db
        .set_default_cost_multiplier(app_type.as_str(), &payload.value)
        .await
        .map_err(|e| ApiError::internal(format!("failed to update default cost multiplier: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_pricing_model_source(
    State(state): State<WebApiState>,
    Path(app): Path<String>,
) -> Result<Json<String>, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    let value = state
        .app_state
        .db
        .get_pricing_model_source(app_type.as_str())
        .await
        .map_err(|e| ApiError::internal(format!("failed to load pricing model source: {e}")))?;
    Ok(Json(value))
}

async fn set_pricing_model_source(
    State(state): State<WebApiState>,
    Path(app): Path<String>,
    Json(payload): Json<ValueRequest>,
) -> Result<StatusCode, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    state
        .app_state
        .db
        .set_pricing_model_source(app_type.as_str(), &payload.value)
        .await
        .map_err(|e| ApiError::internal(format!("failed to update pricing model source: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_provider_health(
    State(state): State<WebApiState>,
    Path((app, provider_id)): Path<(String, String)>,
) -> Result<Json<ProviderHealth>, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    let value = state
        .app_state
        .db
        .get_provider_health(&provider_id, app_type.as_str())
        .await
        .map_err(|e| ApiError::internal(format!("failed to load provider health: {e}")))?;
    Ok(Json(value))
}

async fn get_circuit_breaker_config(
    State(state): State<WebApiState>,
) -> Result<Json<CircuitBreakerConfig>, ApiError> {
    let value = state
        .app_state
        .db
        .get_circuit_breaker_config()
        .await
        .map_err(|e| ApiError::internal(format!("failed to load circuit breaker config: {e}")))?;
    Ok(Json(value))
}

async fn update_circuit_breaker_config(
    State(state): State<WebApiState>,
    Json(config): Json<CircuitBreakerConfig>,
) -> Result<StatusCode, ApiError> {
    state
        .app_state
        .db
        .update_circuit_breaker_config(&config)
        .await
        .map_err(|e| {
            ApiError::internal(format!("failed to update circuit breaker config: {e}"))
        })?;

    state
        .app_state
        .proxy_service
        .update_circuit_breaker_configs(config)
        .await
        .map_err(|e| {
            ApiError::internal(format!("failed to hot reload circuit breaker config: {e}"))
        })?;

    Ok(StatusCode::NO_CONTENT)
}

async fn get_circuit_breaker_stats(
    Path((_app, _provider_id)): Path<(String, String)>,
) -> Json<Option<CircuitBreakerStats>> {
    Json(None)
}

async fn get_failover_queue(
    State(state): State<WebApiState>,
    Path(app): Path<String>,
) -> Result<Json<Vec<FailoverQueueItem>>, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    let queue = state
        .app_state
        .db
        .get_failover_queue(app_type.as_str())
        .map_err(|e| ApiError::internal(format!("failed to load failover queue: {e}")))?;
    Ok(Json(queue))
}

async fn get_available_providers_for_failover(
    State(state): State<WebApiState>,
    Path(app): Path<String>,
) -> Result<Json<Vec<Provider>>, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    let providers = state
        .app_state
        .db
        .get_available_providers_for_failover(app_type.as_str())
        .map_err(|e| {
            ApiError::internal(format!(
                "failed to load available providers for failover: {e}"
            ))
        })?;
    Ok(Json(providers))
}

async fn add_to_failover_queue(
    State(state): State<WebApiState>,
    Path(app): Path<String>,
    Json(payload): Json<ProviderIdRequest>,
) -> Result<StatusCode, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    state
        .app_state
        .db
        .add_to_failover_queue(app_type.as_str(), &payload.provider_id)
        .map_err(|e| ApiError::internal(format!("failed to add provider to failover queue: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn remove_from_failover_queue(
    State(state): State<WebApiState>,
    Path((app, provider_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    state
        .app_state
        .db
        .remove_from_failover_queue(app_type.as_str(), &provider_id)
        .map_err(|e| {
            ApiError::internal(format!("failed to remove provider from failover queue: {e}"))
        })?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_auto_failover_enabled(
    State(state): State<WebApiState>,
    Path(app): Path<String>,
) -> Result<Json<bool>, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    let enabled = state
        .app_state
        .db
        .get_proxy_config_for_app(app_type.as_str())
        .await
        .map(|config| config.auto_failover_enabled)
        .map_err(|e| ApiError::internal(format!("failed to load auto failover status: {e}")))?;
    Ok(Json(enabled))
}

async fn set_auto_failover_enabled(
    State(state): State<WebApiState>,
    Path(app): Path<String>,
    Json(payload): Json<EnabledRequest>,
) -> Result<StatusCode, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    let app_type_str = app_type.as_str();

    let p1_provider_id = if payload.enabled {
        let mut queue = state
            .app_state
            .db
            .get_failover_queue(app_type_str)
            .map_err(|e| ApiError::internal(format!("failed to load failover queue: {e}")))?;

        if queue.is_empty() {
            let current_id =
                crate::settings::get_effective_current_provider(state.app_state.db.as_ref(), &app_type)
                    .map_err(|e| {
                        ApiError::internal(format!(
                            "failed to get current provider for failover: {e}"
                        ))
                    })?;

            let Some(current_id) = current_id else {
                return Err(ApiError::bad_request(
                    "故障转移队列为空，且未设置当前供应商，无法开启故障转移",
                ));
            };

            state
                .app_state
                .db
                .add_to_failover_queue(app_type_str, &current_id)
                .map_err(|e| {
                    ApiError::internal(format!(
                        "failed to add current provider into failover queue: {e}"
                    ))
                })?;

            queue = state
                .app_state
                .db
                .get_failover_queue(app_type_str)
                .map_err(|e| ApiError::internal(format!("failed to reload failover queue: {e}")))?;
        }

        Some(
            queue
                .first()
                .map(|item| item.provider_id.clone())
                .ok_or_else(|| ApiError::bad_request("故障转移队列为空，无法开启故障转移"))?,
        )
    } else {
        None
    };

    let mut config = state
        .app_state
        .db
        .get_proxy_config_for_app(app_type_str)
        .await
        .map_err(|e| ApiError::internal(format!("failed to load app proxy config: {e}")))?;
    config.auto_failover_enabled = payload.enabled;
    state
        .app_state
        .db
        .update_proxy_config_for_app(config)
        .await
        .map_err(|e| ApiError::internal(format!("failed to update auto failover config: {e}")))?;

    if let Some(provider_id) = p1_provider_id {
        state
            .app_state
            .proxy_service
            .switch_proxy_target(app_type_str, &provider_id)
            .await
            .map_err(|e| {
                ApiError::internal(format!("failed to switch proxy target for failover: {e}"))
            })?;
    }

    Ok(StatusCode::NO_CONTENT)
}

fn resolve_bind_addr() -> Result<SocketAddr, String> {
    let host = std::env::var("CC_SWITCH_WEB_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("CC_SWITCH_WEB_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(8788);

    let ip = host
        .parse::<IpAddr>()
        .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));

    Ok(SocketAddr::new(ip, port))
}

fn resolve_frontend_dist_dir() -> Option<PathBuf> {
    let configured = std::env::var("CC_SWITCH_WEB_DIST_DIR")
        .ok()
        .map(PathBuf::from);

    let dist_dir = configured.unwrap_or_else(|| PathBuf::from("dist"));

    if dist_dir.exists() {
        Some(dist_dir)
    } else {
        None
    }
}

pub async fn run_web_server() -> Result<(), String> {
    let db = Arc::new(Database::init().map_err(|e| format!("database init failed: {e}"))?);
    let app_state = Arc::new(AppState::new(db));
    let state = WebApiState { app_state };
    let bind_addr = resolve_bind_addr()?;

    let mut app = Router::new()
        .route("/", get(root))
        .route("/api/health", get(health))
        .route("/api/settings", get(get_settings).put(save_settings))
        .route(
            "/api/settings/rectifier",
            get(get_rectifier_config).put(set_rectifier_config),
        )
        .route(
            "/api/settings/optimizer",
            get(get_optimizer_config).put(set_optimizer_config),
        )
        .route("/api/providers/:app", get(get_providers).post(add_provider))
        .route("/api/providers/:app/current", get(get_current_provider))
        .route("/api/providers/:app/live-provider-ids", get(get_live_provider_ids))
        .route("/api/providers/:app/import-live", post(import_providers_from_live))
        .route(
            "/api/providers/:app/live-config/:id",
            axum::routing::delete(remove_provider_from_live_config),
        )
        .route("/api/skills/installed", get(get_installed_skills))
        .route("/api/skills/backups", get(get_skill_backups))
        .route("/api/skills/unmanaged", get(scan_unmanaged_skills))
        .route("/api/skills/import", post(import_skills_from_apps))
        .route("/api/skills/repos", get(get_skill_repos).post(add_skill_repo))
        .route(
            "/api/skills/repos/:owner/:name",
            axum::routing::delete(remove_skill_repo),
        )
        .route("/api/skills/discover", get(discover_available_skills))
        .route("/api/skills/install", post(install_skill_unified))
        .route("/api/skills/install-archives", post(install_skill_archives))
        .route(
            "/api/skills/backups/:backup_id",
            post(restore_skill_backup).delete(delete_skill_backup),
        )
        .route(
            "/api/skills/:id",
            axum::routing::delete(uninstall_skill_unified),
        )
        .route("/api/skills/:id/apps/:app", put(toggle_skill_app))
        .route(
            "/api/workspace/files/:filename",
            get(get_workspace_file).put(save_workspace_file),
        )
        .route(
            "/api/workspace/daily-memory",
            get(list_workspace_daily_memory_files),
        )
        .route(
            "/api/workspace/daily-memory/search",
            get(search_workspace_daily_memory_files),
        )
        .route(
            "/api/workspace/daily-memory/:filename",
            get(get_workspace_daily_memory_file)
                .put(save_workspace_daily_memory_file)
                .delete(delete_workspace_daily_memory_file),
        )
        .route(
            "/api/workspace/directories/:subdir/path",
            get(get_workspace_directory_path),
        )
        .route(
            "/api/openclaw/default-model",
            get(get_openclaw_default_model).put(set_openclaw_default_model),
        )
        .route(
            "/api/openclaw/model-catalog",
            get(get_openclaw_model_catalog).put(set_openclaw_model_catalog),
        )
        .route(
            "/api/openclaw/agents-defaults",
            get(get_openclaw_agents_defaults).put(set_openclaw_agents_defaults),
        )
        .route("/api/openclaw/env", get(get_openclaw_env).put(set_openclaw_env))
        .route("/api/openclaw/tools", get(get_openclaw_tools).put(set_openclaw_tools))
        .route("/api/openclaw/health", get(scan_openclaw_config_health))
        .route(
            "/api/openclaw/live-provider/:provider_id",
            get(get_openclaw_live_provider),
        )
        .route("/api/sessions", get(list_sessions).delete(delete_session))
        .route("/api/sessions/messages", get(get_session_messages))
        .route("/api/sessions/delete-batch", post(delete_sessions))
        .route("/api/usage/summary", get(get_usage_summary))
        .route("/api/usage/trends", get(get_usage_trends))
        .route("/api/usage/provider-stats", get(get_usage_provider_stats))
        .route("/api/usage/model-stats", get(get_usage_model_stats))
        .route("/api/usage/request-logs", post(get_usage_request_logs))
        .route(
            "/api/usage/request-logs/:request_id",
            get(get_usage_request_detail),
        )
        .route("/api/usage/model-pricing", get(get_usage_model_pricing))
        .route(
            "/api/usage/model-pricing/:model_id",
            put(update_usage_model_pricing).delete(delete_usage_model_pricing),
        )
        .route(
            "/api/usage/provider-limits/:app_type/:provider_id",
            get(get_usage_provider_limits),
        )
        .route("/api/prompts/:app", get(get_prompts))
        .route("/api/prompts/:app/import", post(import_prompt_from_file))
        .route(
            "/api/prompts/:app/current-file",
            get(get_current_prompt_file_content),
        )
        .route(
            "/api/prompts/:app/:id",
            put(upsert_prompt).delete(delete_prompt),
        )
        .route("/api/prompts/:app/:id/enable", post(enable_prompt))
        .route("/api/mcp/servers", get(get_mcp_servers).post(upsert_mcp_server))
        .route("/api/mcp/servers/import", post(import_mcp_from_apps))
        .route("/api/mcp/servers/:id", axum::routing::delete(delete_mcp_server))
        .route("/api/mcp/servers/:id/apps/:app", put(toggle_mcp_app))
        .route(
            "/api/providers/:app/:id",
            put(update_provider).delete(delete_provider),
        )
        .route("/api/providers/:app/:id/switch", post(switch_provider))
        .route("/api/proxy/status", get(get_proxy_status))
        .route("/api/proxy/takeover-status", get(get_proxy_takeover_status))
        .route(
            "/api/proxy/config",
            get(get_proxy_config).put(update_proxy_config),
        )
        .route(
            "/api/proxy/global-config",
            get(get_global_proxy_config).put(update_global_proxy_config),
        )
        .route(
            "/api/proxy/apps/:app/config",
            get(get_proxy_config_for_app).put(update_proxy_config_for_app),
        )
        .route("/api/proxy/running", get(is_proxy_running))
        .route(
            "/api/proxy/live-takeover-active",
            get(is_live_takeover_active),
        )
        .route("/api/proxy/start", post(start_proxy_server))
        .route("/api/proxy/stop-with-restore", post(stop_proxy_with_restore))
        .route(
            "/api/proxy/apps/:app/takeover",
            put(set_proxy_takeover_for_app),
        )
        .route(
            "/api/proxy/apps/:app/providers/:id/switch",
            post(switch_proxy_provider),
        )
        .route(
            "/api/proxy/apps/:app/default-cost-multiplier",
            get(get_default_cost_multiplier).put(set_default_cost_multiplier),
        )
        .route(
            "/api/proxy/apps/:app/pricing-model-source",
            get(get_pricing_model_source).put(set_pricing_model_source),
        )
        .route(
            "/api/failover/apps/:app/queue",
            get(get_failover_queue).post(add_to_failover_queue),
        )
        .route(
            "/api/failover/apps/:app/queue/:provider_id",
            axum::routing::delete(remove_from_failover_queue),
        )
        .route(
            "/api/failover/apps/:app/available-providers",
            get(get_available_providers_for_failover),
        )
        .route(
            "/api/failover/apps/:app/auto-enabled",
            get(get_auto_failover_enabled).put(set_auto_failover_enabled),
        )
        .route(
            "/api/failover/apps/:app/providers/:provider_id/health",
            get(get_provider_health),
        )
        .route(
            "/api/failover/circuit-breaker-config",
            get(get_circuit_breaker_config).put(update_circuit_breaker_config),
        )
        .route(
            "/api/failover/apps/:app/providers/:provider_id/circuit-breaker-stats",
            get(get_circuit_breaker_stats),
        )
        .layer(DefaultBodyLimit::max(128 * 1024 * 1024))
        .layer(CorsLayer::permissive())
        .with_state(state);

    if let Some(dist_dir) = resolve_frontend_dist_dir() {
        println!(
            "cc-switch web service will serve static assets from {}",
            dist_dir.display()
        );
        app = app.fallback_service(get_service(
            ServeDir::new(dist_dir).append_index_html_on_directories(true),
        ));
    } else {
        println!("cc-switch web service running without frontend static assets");
    }

    println!("cc-switch web service listening on http://{bind_addr}");

    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .map_err(|e| format!("bind failed on {bind_addr}: {e}"))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| format!("server error: {e}"))
}
