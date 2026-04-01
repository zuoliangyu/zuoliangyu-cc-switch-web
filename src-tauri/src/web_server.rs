use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{DefaultBodyLimit, Multipart, Path, Query, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, get_service, post, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

use crate::app_config::{AppType, McpServer};
use crate::database::FailoverQueueItem;
use crate::prompt::Prompt;
use crate::provider::Provider;
use crate::proxy::http_client;
use crate::proxy::circuit_breaker::{CircuitBreakerConfig, CircuitBreakerStats};
use crate::proxy::providers::copilot_auth::CopilotAuthManager;
use crate::proxy::types::{
    AppProxyConfig, GlobalProxyConfig, LogConfig, OptimizerConfig, ProviderHealth, ProxyConfig,
    ProxyServerInfo, ProxyStatus, ProxyTakeoverStatus, RectifierConfig,
};
use crate::services::omo::{OmoLocalFileData, OmoService, SLIM, STANDARD};
use crate::services::skill::{
    DiscoverableSkill, ImportSkillSelection, SkillBackupEntry, SkillRepo, SkillUninstallResult,
};
use crate::services::webdav_sync as webdav_sync_service;
use crate::services::{McpService, PromptService, ProviderService, SwitchResult};
use crate::settings::{self, WebDavSyncSettings};
use crate::store::AppState;
use crate::Database;
use tokio::sync::RwLock;

#[derive(Clone)]
struct WebApiState {
    app_state: Arc<AppState>,
    copilot_auth_state: Arc<RwLock<CopilotAuthManager>>,
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
struct OptionalPathRequest {
    path: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnippetRequest {
    snippet: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExtractCommonConfigSnippetRequest {
    settings_config: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderIdRequest {
    provider_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestUsageScriptRequest {
    script_code: String,
    timeout: Option<u64>,
    api_key: Option<String>,
    base_url: Option<String>,
    access_token: Option<String>,
    user_id: Option<String>,
    template_type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EndpointTestRequest {
    urls: Vec<String>,
    timeout_secs: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StreamCheckAllProvidersRequest {
    proxy_targets_only: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CustomEndpointUrlRequest {
    url: String,
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
struct RenameBackupRequest {
    old_filename: String,
    new_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportConfigQuery {
    filename: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WebdavTestRequest {
    settings: WebDavSyncSettings,
    preserve_empty_password: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WebdavSaveSettingsRequest {
    settings: WebDavSyncSettings,
    password_touched: Option<bool>,
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
struct ToolVersionsRequest {
    tools: Option<Vec<String>>,
    wsl_shell_by_tool: Option<HashMap<String, crate::commands::WslShellPreferenceInput>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthStartLoginRequest {
    auth_provider: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthPollForAccountRequest {
    auth_provider: String,
    device_code: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthAccountRequest {
    auth_provider: String,
    account_id: String,
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

fn sanitize_export_sql_filename(filename: Option<String>) -> String {
    let fallback = "cc-switch-export.sql".to_string();
    let mut sanitized = filename
        .unwrap_or_else(|| fallback.clone())
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ if ch.is_control() => '_',
            _ => ch,
        })
        .collect::<String>();

    if sanitized.trim().is_empty() {
        sanitized = fallback;
    }

    if !sanitized.to_ascii_lowercase().ends_with(".sql") {
        sanitized.push_str(".sql");
    }

    sanitized
}

fn build_post_import_sync_warning(error: impl std::fmt::Display) -> String {
    crate::error::AppError::localized(
        "sync.post_operation_sync_failed",
        format!("后置同步状态失败: {error}"),
        format!("Post-operation synchronization failed: {error}"),
    )
    .to_string()
}

fn attach_warning_to_value(mut value: Value, warning: Option<String>) -> Value {
    if let Some(message) = warning {
        if let Some(object) = value.as_object_mut() {
            object.insert("warning".to_string(), Value::String(message));
        }
    }
    value
}

fn persist_webdav_sync_error(
    settings: &mut WebDavSyncSettings,
    error: &crate::error::AppError,
    source: &str,
) {
    settings.status.last_error = Some(error.to_string());
    settings.status.last_error_source = Some(source.to_string());
    let _ = settings::update_webdav_sync_status(settings.status.clone());
}

fn webdav_not_configured_error() -> ApiError {
    ApiError::bad_request(
        crate::error::AppError::localized(
            "webdav.sync.not_configured",
            "未配置 WebDAV 同步",
            "WebDAV sync is not configured.",
        )
        .to_string(),
    )
}

fn webdav_sync_disabled_error() -> ApiError {
    ApiError::bad_request(
        crate::error::AppError::localized(
            "webdav.sync.disabled",
            "WebDAV 同步未启用",
            "WebDAV sync is disabled.",
        )
        .to_string(),
    )
}

fn require_enabled_webdav_settings() -> Result<WebDavSyncSettings, ApiError> {
    let sync_settings =
        settings::get_webdav_sync_settings().ok_or_else(webdav_not_configured_error)?;
    if !sync_settings.enabled {
        return Err(webdav_sync_disabled_error());
    }
    Ok(sync_settings)
}

fn resolve_webdav_password_for_request(
    mut incoming: WebDavSyncSettings,
    existing: Option<WebDavSyncSettings>,
    preserve_empty_password: bool,
) -> WebDavSyncSettings {
    if let Some(existing_settings) = existing {
        if preserve_empty_password && incoming.password.is_empty() {
            incoming.password = existing_settings.password;
        }
    }
    incoming
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

async fn get_live_provider_ids(Path(app): Path<String>) -> Result<Json<Vec<String>>, ApiError> {
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
    crate::services::ProviderService::remove_from_live_config(
        state.app_state.as_ref(),
        app_type,
        &id,
    )
    .map_err(|e| ApiError::internal(format!("failed to remove provider from live config: {e}")))?;
    Ok(Json(true))
}

async fn import_default_provider_config(
    State(state): State<WebApiState>,
    Path(app): Path<String>,
) -> Result<Json<bool>, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    let imported =
        crate::commands::import_default_config_test_hook(state.app_state.as_ref(), app_type)
            .map_err(|e| ApiError::internal(format!("failed to import default config: {e}")))?;
    Ok(Json(imported))
}

async fn stream_check_provider(
    State(state): State<WebApiState>,
    Path((app, id)): Path<(String, String)>,
) -> Result<Json<crate::services::stream_check::StreamCheckResult>, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    let result = crate::commands::stream_check_provider_internal(
        state.app_state.as_ref(),
        &state.copilot_auth_state,
        app_type,
        &id,
    )
    .await
    .map_err(|e| ApiError::internal(format!("failed to stream check provider: {e}")))?;
    Ok(Json(result))
}

async fn stream_check_all_providers(
    State(state): State<WebApiState>,
    Path(app): Path<String>,
    Json(payload): Json<StreamCheckAllProvidersRequest>,
) -> Result<Json<Vec<(String, crate::services::stream_check::StreamCheckResult)>>, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    let result = crate::commands::stream_check_all_providers_internal(
        state.app_state.as_ref(),
        &state.copilot_auth_state,
        app_type,
        payload.proxy_targets_only,
    )
    .await
    .map_err(|e| ApiError::internal(format!("failed to stream check all providers: {e}")))?;
    Ok(Json(result))
}

async fn read_live_provider_settings(
    Path(app): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    let settings = ProviderService::read_live_settings(app_type)
        .map_err(|e| ApiError::internal(format!("failed to read live provider settings: {e}")))?;
    Ok(Json(settings))
}

async fn query_provider_usage(
    State(state): State<WebApiState>,
    Path((app, id)): Path<(String, String)>,
) -> Result<Json<crate::provider::UsageResult>, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    let result = crate::commands::query_provider_usage_internal(
        state.app_state.as_ref(),
        &state.copilot_auth_state,
        app_type,
        &id,
    )
    .await
    .map_err(|e| ApiError::internal(format!("failed to query provider usage: {e}")))?;
    Ok(Json(result))
}

async fn test_usage_script(
    State(state): State<WebApiState>,
    Path((app, id)): Path<(String, String)>,
    Json(payload): Json<TestUsageScriptRequest>,
) -> Result<Json<crate::provider::UsageResult>, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    let result = crate::commands::test_usage_script_internal(
        state.app_state.as_ref(),
        app_type,
        &id,
        &payload.script_code,
        payload.timeout,
        payload.api_key.as_deref(),
        payload.base_url.as_deref(),
        payload.access_token.as_deref(),
        payload.user_id.as_deref(),
        payload.template_type.as_deref(),
    )
    .await
    .map_err(|e| ApiError::internal(format!("failed to test usage script: {e}")))?;
    Ok(Json(result))
}

async fn test_api_endpoints(
    Json(payload): Json<EndpointTestRequest>,
) -> Result<Json<Vec<crate::services::EndpointLatency>>, ApiError> {
    let results =
        crate::services::SpeedtestService::test_endpoints(payload.urls, payload.timeout_secs)
            .await
            .map_err(|e| ApiError::internal(format!("failed to test api endpoints: {e}")))?;
    Ok(Json(results))
}

async fn get_custom_endpoints(
    State(state): State<WebApiState>,
    Path((app, id)): Path<(String, String)>,
) -> Result<Json<Vec<crate::settings::CustomEndpoint>>, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    let endpoints = ProviderService::get_custom_endpoints(state.app_state.as_ref(), app_type, &id)
        .map_err(|e| ApiError::internal(format!("failed to load custom endpoints: {e}")))?;
    Ok(Json(endpoints))
}

async fn add_custom_endpoint(
    State(state): State<WebApiState>,
    Path((app, id)): Path<(String, String)>,
    Json(payload): Json<CustomEndpointUrlRequest>,
) -> Result<StatusCode, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    ProviderService::add_custom_endpoint(state.app_state.as_ref(), app_type, &id, payload.url)
        .map_err(|e| ApiError::internal(format!("failed to add custom endpoint: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn remove_custom_endpoint(
    State(state): State<WebApiState>,
    Path((app, id)): Path<(String, String)>,
    Json(payload): Json<CustomEndpointUrlRequest>,
) -> Result<StatusCode, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    ProviderService::remove_custom_endpoint(state.app_state.as_ref(), app_type, &id, payload.url)
        .map_err(|e| ApiError::internal(format!("failed to remove custom endpoint: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn update_endpoint_last_used(
    State(state): State<WebApiState>,
    Path((app, id)): Path<(String, String)>,
    Json(payload): Json<CustomEndpointUrlRequest>,
) -> Result<StatusCode, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    ProviderService::update_endpoint_last_used(
        state.app_state.as_ref(),
        app_type,
        &id,
        payload.url,
    )
    .map_err(|e| ApiError::internal(format!("failed to update endpoint last used: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_universal_providers(
    State(state): State<WebApiState>,
) -> Result<Json<HashMap<String, crate::provider::UniversalProvider>>, ApiError> {
    let providers = ProviderService::list_universal(state.app_state.as_ref())
        .map_err(|e| ApiError::internal(format!("failed to load universal providers: {e}")))?;
    Ok(Json(providers))
}

async fn get_universal_provider(
    State(state): State<WebApiState>,
    Path(id): Path<String>,
) -> Result<Json<Option<crate::provider::UniversalProvider>>, ApiError> {
    let provider = ProviderService::get_universal(state.app_state.as_ref(), &id)
        .map_err(|e| ApiError::internal(format!("failed to load universal provider: {e}")))?;
    Ok(Json(provider))
}

async fn upsert_universal_provider(
    State(state): State<WebApiState>,
    Json(provider): Json<crate::provider::UniversalProvider>,
) -> Result<Json<bool>, ApiError> {
    let result = ProviderService::upsert_universal(state.app_state.as_ref(), provider)
        .map_err(|e| ApiError::internal(format!("failed to save universal provider: {e}")))?;
    Ok(Json(result))
}

async fn delete_universal_provider(
    State(state): State<WebApiState>,
    Path(id): Path<String>,
) -> Result<Json<bool>, ApiError> {
    let result = ProviderService::delete_universal(state.app_state.as_ref(), &id)
        .map_err(|e| ApiError::internal(format!("failed to delete universal provider: {e}")))?;
    Ok(Json(result))
}

async fn sync_universal_provider(
    State(state): State<WebApiState>,
    Path(id): Path<String>,
) -> Result<Json<bool>, ApiError> {
    let result = ProviderService::sync_universal_to_apps(state.app_state.as_ref(), &id)
        .map_err(|e| ApiError::internal(format!("failed to sync universal provider: {e}")))?;
    Ok(Json(result))
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
    let skills =
        crate::services::skill::SkillService::import_from_apps(&state.app_state.db, imports)
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

async fn delete_skill_backup(Path(backup_id): Path<String>) -> Result<Json<bool>, ApiError> {
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

async fn get_openclaw_model_catalog() -> Result<
    Json<Option<HashMap<String, crate::openclaw_config::OpenClawModelCatalogEntry>>>,
    ApiError,
> {
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
    let outcome = crate::openclaw_config::set_agents_defaults(&defaults)
        .map_err(|e| ApiError::internal(format!("failed to save openclaw agents defaults: {e}")))?;
    Ok(Json(outcome))
}

async fn get_openclaw_env() -> Result<Json<crate::openclaw_config::OpenClawEnvConfig>, ApiError> {
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

async fn get_openclaw_tools() -> Result<Json<crate::openclaw_config::OpenClawToolsConfig>, ApiError>
{
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
    let deleted =
        crate::delete_session(payload.provider_id, payload.session_id, payload.source_path)
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
    let app_type =
        AppType::from_str(&current_app).map_err(|e| ApiError::bad_request(e.to_string()))?;

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

        let install_result = crate::services::skill::SkillService::install_from_zip(
            &state.app_state.db,
            &archive_path,
            &app_type,
        );

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
        (
            self.status,
            Json(ErrorResponse {
                error: self.message,
            }),
        )
            .into_response()
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

async fn export_config_download(
    State(state): State<WebApiState>,
    Query(query): Query<ExportConfigQuery>,
) -> Result<Response, ApiError> {
    let sql = state
        .app_state
        .db
        .export_sql_string()
        .map_err(|e| ApiError::internal(format!("failed to export config: {e}")))?;
    let filename = sanitize_export_sql_filename(query.filename);
    let disposition = HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
        .map_err(|e| ApiError::internal(format!("failed to encode download filename: {e}")))?;

    Ok((
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/sql; charset=utf-8"),
            ),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        Body::from(sql.into_bytes()),
    )
        .into_response())
}

async fn import_config_upload(
    State(state): State<WebApiState>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut file_name: Option<String> = None;
    let mut sql_raw: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::bad_request(format!("failed to read upload field: {e}")))?
    {
        if field.name() != Some("file") {
            continue;
        }

        let current_file_name = field
            .file_name()
            .unwrap_or("cc-switch-import.sql")
            .to_string();
        let bytes = field.bytes().await.map_err(|e| {
            ApiError::bad_request(format!(
                "failed to read uploaded config file {current_file_name}: {e}"
            ))
        })?;

        let sql = String::from_utf8(bytes.to_vec()).map_err(|e| {
            ApiError::bad_request(format!(
                "uploaded config file must be valid UTF-8 SQL text: {e}"
            ))
        })?;

        file_name = Some(current_file_name);
        sql_raw = Some(sql);
        break;
    }

    let file_name = file_name.unwrap_or_else(|| "cc-switch-import.sql".to_string());
    if !file_name.to_ascii_lowercase().ends_with(".sql") {
        return Err(ApiError::bad_request("only .sql files are supported"));
    }

    let backup_id = state
        .app_state
        .db
        .import_sql_string(
            sql_raw
                .as_deref()
                .ok_or_else(|| ApiError::bad_request("missing uploaded config file"))?,
        )
        .map_err(|e| ApiError::internal(format!("failed to import config: {e}")))?;

    let warning = match ProviderService::sync_current_to_live(state.app_state.as_ref()) {
        Ok(()) => crate::settings::reload_settings()
            .err()
            .map(build_post_import_sync_warning),
        Err(error) => Some(build_post_import_sync_warning(error)),
    };
    if let Some(message) = warning.as_ref() {
        log::warn!("[Import] post-import sync warning: {message}");
    }

    let mut payload = json!({
        "success": true,
        "message": "SQL imported successfully",
        "backupId": backup_id,
    });
    if let Some(message) = warning {
        if let Some(object) = payload.as_object_mut() {
            object.insert("warning".to_string(), Value::String(message));
        }
    }

    Ok(Json(payload))
}

async fn webdav_test_connection(
    Json(payload): Json<WebdavTestRequest>,
) -> Result<Json<Value>, ApiError> {
    let preserve_empty = payload.preserve_empty_password.unwrap_or(true);
    let resolved = resolve_webdav_password_for_request(
        payload.settings,
        settings::get_webdav_sync_settings(),
        preserve_empty,
    );
    webdav_sync_service::check_connection(&resolved)
        .await
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    Ok(Json(json!({
        "success": true,
        "message": "WebDAV connection ok",
    })))
}

async fn webdav_sync_upload(State(state): State<WebApiState>) -> Result<Json<Value>, ApiError> {
    let db = state.app_state.db.clone();
    let mut sync_settings = require_enabled_webdav_settings()?;
    let result = webdav_sync_service::run_with_sync_lock(webdav_sync_service::upload(
        &db,
        &mut sync_settings,
    ))
    .await;

    match result {
        Ok(value) => Ok(Json(value)),
        Err(error) => {
            persist_webdav_sync_error(&mut sync_settings, &error, "manual");
            Err(ApiError::internal(error.to_string()))
        }
    }
}

async fn webdav_sync_download(State(state): State<WebApiState>) -> Result<Json<Value>, ApiError> {
    let db = state.app_state.db.clone();
    let mut sync_settings = require_enabled_webdav_settings()?;
    let _auto_sync_suppression = crate::services::webdav_auto_sync::AutoSyncSuppressionGuard::new();

    let result = webdav_sync_service::run_with_sync_lock(webdav_sync_service::download(
        &db,
        &mut sync_settings,
    ))
    .await;

    let mut value = match result {
        Ok(value) => value,
        Err(error) => {
            persist_webdav_sync_error(&mut sync_settings, &error, "manual");
            return Err(ApiError::internal(error.to_string()));
        }
    };

    let warning = match ProviderService::sync_current_to_live(state.app_state.as_ref()) {
        Ok(()) => settings::reload_settings()
            .err()
            .map(build_post_import_sync_warning),
        Err(error) => Some(build_post_import_sync_warning(error)),
    };
    if let Some(message) = warning.as_ref() {
        log::warn!("[WebDAV] post-download sync warning: {message}");
    }
    value = attach_warning_to_value(value, warning);

    Ok(Json(value))
}

async fn webdav_sync_save_settings(
    Json(payload): Json<WebdavSaveSettingsRequest>,
) -> Result<Json<Value>, ApiError> {
    let password_touched = payload.password_touched.unwrap_or(false);
    let existing = settings::get_webdav_sync_settings();
    let mut sync_settings =
        resolve_webdav_password_for_request(payload.settings, existing.clone(), !password_touched);

    if let Some(existing_settings) = existing {
        sync_settings.status = existing_settings.status;
    }

    sync_settings.normalize();
    sync_settings
        .validate()
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    settings::set_webdav_sync_settings(Some(sync_settings))
        .map_err(|e| ApiError::internal(format!("failed to save webdav sync settings: {e}")))?;
    Ok(Json(json!({ "success": true })))
}

async fn webdav_sync_fetch_remote_info() -> Result<Json<Value>, ApiError> {
    let sync_settings = require_enabled_webdav_settings()?;
    let info = webdav_sync_service::fetch_remote_info(&sync_settings)
        .await
        .map_err(|e| ApiError::internal(format!("failed to fetch webdav remote info: {e}")))?;
    Ok(Json(info.unwrap_or(json!({ "empty": true }))))
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

async fn get_app_config_dir_override() -> Json<Option<String>> {
    Json(
        crate::app_store::refresh_app_config_dir_override()
            .map(|path| path.to_string_lossy().to_string()),
    )
}

async fn set_app_config_dir_override(
    Json(payload): Json<OptionalPathRequest>,
) -> Result<Json<bool>, ApiError> {
    crate::app_store::set_app_config_dir_override(payload.path.as_deref())
        .map_err(|e| ApiError::internal(format!("failed to save app config dir override: {e}")))?;
    Ok(Json(true))
}

async fn get_common_config_snippet(
    State(state): State<WebApiState>,
    Path(app): Path<String>,
) -> Result<Json<Option<String>>, ApiError> {
    let snippet = state
        .app_state
        .db
        .get_config_snippet(&app)
        .map_err(|e| ApiError::internal(format!("failed to load common config snippet: {e}")))?;
    Ok(Json(snippet))
}

async fn set_common_config_snippet(
    State(state): State<WebApiState>,
    Path(app): Path<String>,
    Json(payload): Json<SnippetRequest>,
) -> Result<StatusCode, ApiError> {
    let is_cleared = payload.snippet.trim().is_empty();
    let old_snippet = state
        .app_state
        .db
        .get_config_snippet(&app)
        .map_err(|e| ApiError::internal(format!("failed to load current common config snippet: {e}")))?;

    crate::validate_common_config_snippet(&app, &payload.snippet).map_err(ApiError::bad_request)?;

    let value = if is_cleared {
        None
    } else {
        Some(payload.snippet.clone())
    };

    if matches!(app.as_str(), "claude" | "codex" | "gemini") {
        if let Some(legacy_snippet) = old_snippet
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
            ProviderService::migrate_legacy_common_config_usage(
                state.app_state.as_ref(),
                app_type,
                legacy_snippet,
            )
            .map_err(|e| {
                ApiError::internal(format!("failed to migrate legacy common config usage: {e}"))
            })?;
        }
    }

    state
        .app_state
        .db
        .set_config_snippet(&app, value)
        .map_err(|e| ApiError::internal(format!("failed to save common config snippet: {e}")))?;
    state
        .app_state
        .db
        .set_config_snippet_cleared(&app, is_cleared)
        .map_err(|e| ApiError::internal(format!("failed to save common config cleared state: {e}")))?;

    if matches!(app.as_str(), "claude" | "codex" | "gemini") {
        let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
        ProviderService::sync_current_provider_for_app(state.app_state.as_ref(), app_type)
            .map_err(|e| {
                ApiError::internal(format!("failed to sync current provider after snippet update: {e}"))
            })?;
    }

    if app == "omo"
        && state
            .app_state
            .db
            .get_current_omo_provider("opencode", "omo")
            .map_err(|e| ApiError::internal(format!("failed to load current OMO provider: {e}")))?
            .is_some()
    {
        OmoService::write_config_to_file(state.app_state.as_ref(), &STANDARD)
            .map_err(|e| ApiError::internal(format!("failed to write OMO config after snippet update: {e}")))?;
    }

    if app == "omo-slim"
        && state
            .app_state
            .db
            .get_current_omo_provider("opencode", "omo-slim")
            .map_err(|e| ApiError::internal(format!("failed to load current OMO Slim provider: {e}")))?
            .is_some()
    {
        OmoService::write_config_to_file(state.app_state.as_ref(), &SLIM)
            .map_err(|e| {
                ApiError::internal(format!(
                    "failed to write OMO Slim config after snippet update: {e}"
                ))
            })?;
    }

    Ok(StatusCode::NO_CONTENT)
}

async fn extract_common_config_snippet(
    State(state): State<WebApiState>,
    Path(app): Path<String>,
    Json(payload): Json<ExtractCommonConfigSnippetRequest>,
) -> Result<Json<String>, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;

    if let Some(settings_config) = payload
        .settings_config
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        let settings: serde_json::Value = serde_json::from_str(settings_config)
            .map_err(|e| ApiError::bad_request(crate::invalid_json_format_error(e)))?;
        let snippet =
            ProviderService::extract_common_config_snippet_from_settings(app_type, &settings)
                .map_err(|e| {
                    ApiError::internal(format!(
                        "failed to extract common config snippet from settings: {e}"
                    ))
                })?;
        return Ok(Json(snippet));
    }

    let snippet = ProviderService::extract_common_config_snippet(state.app_state.as_ref(), app_type)
        .map_err(|e| {
            ApiError::internal(format!(
                "failed to extract common config snippet from current provider: {e}"
            ))
        })?;
    Ok(Json(snippet))
}

async fn sync_current_providers_live(
    State(state): State<WebApiState>,
) -> Result<Json<Value>, ApiError> {
    ProviderService::sync_current_to_live(state.app_state.as_ref())
        .map_err(|e| ApiError::internal(format!("failed to sync current providers to live: {e}")))?;
    Ok(Json(json!({
        "success": true,
        "message": "Live configuration synchronized",
    })))
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

async fn get_log_config(State(state): State<WebApiState>) -> Result<Json<LogConfig>, ApiError> {
    let config = state
        .app_state
        .db
        .get_log_config()
        .map_err(|e| ApiError::internal(format!("failed to load log config: {e}")))?;
    Ok(Json(config))
}

async fn set_log_config(
    State(state): State<WebApiState>,
    Json(config): Json<LogConfig>,
) -> Result<Json<bool>, ApiError> {
    state
        .app_state
        .db
        .set_log_config(&config)
        .map_err(|e| ApiError::internal(format!("failed to save log config: {e}")))?;
    log::set_max_level(config.to_level_filter());
    Ok(Json(true))
}

async fn get_stream_check_config(
    State(state): State<WebApiState>,
) -> Result<Json<crate::services::stream_check::StreamCheckConfig>, ApiError> {
    let config = state
        .app_state
        .db
        .get_stream_check_config()
        .map_err(|e| ApiError::internal(format!("failed to load stream check config: {e}")))?;
    Ok(Json(config))
}

async fn set_stream_check_config(
    State(state): State<WebApiState>,
    Json(config): Json<crate::services::stream_check::StreamCheckConfig>,
) -> Result<StatusCode, ApiError> {
    state
        .app_state
        .db
        .save_stream_check_config(&config)
        .map_err(|e| ApiError::internal(format!("failed to save stream check config: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_tool_versions(
    Json(payload): Json<ToolVersionsRequest>,
) -> Result<Json<Vec<crate::commands::ToolVersion>>, ApiError> {
    let versions = crate::commands::get_tool_versions(payload.tools, payload.wsl_shell_by_tool)
        .await
        .map_err(|e| ApiError::internal(format!("failed to load tool versions: {e}")))?;
    Ok(Json(versions))
}

async fn auth_start_login(
    State(state): State<WebApiState>,
    Json(payload): Json<AuthStartLoginRequest>,
) -> Result<Json<crate::commands::ManagedAuthDeviceCodeResponse>, ApiError> {
    let response = crate::commands::auth_start_login_internal(
        &payload.auth_provider,
        &state.copilot_auth_state,
    )
    .await
    .map_err(|e| ApiError::internal(format!("failed to start auth login: {e}")))?;
    Ok(Json(response))
}

async fn auth_poll_for_account(
    State(state): State<WebApiState>,
    Json(payload): Json<AuthPollForAccountRequest>,
) -> Result<Json<Option<crate::commands::ManagedAuthAccount>>, ApiError> {
    let account = crate::commands::auth_poll_for_account_internal(
        &payload.auth_provider,
        &payload.device_code,
        &state.copilot_auth_state,
    )
    .await
    .map_err(|e| ApiError::internal(format!("failed to poll auth account: {e}")))?;
    Ok(Json(account))
}

async fn auth_list_accounts(
    State(state): State<WebApiState>,
    Path(auth_provider): Path<String>,
) -> Result<Json<Vec<crate::commands::ManagedAuthAccount>>, ApiError> {
    let accounts =
        crate::commands::auth_list_accounts_internal(&auth_provider, &state.copilot_auth_state)
            .await
            .map_err(|e| ApiError::internal(format!("failed to list auth accounts: {e}")))?;
    Ok(Json(accounts))
}

async fn auth_get_status(
    State(state): State<WebApiState>,
    Path(auth_provider): Path<String>,
) -> Result<Json<crate::commands::ManagedAuthStatus>, ApiError> {
    let status =
        crate::commands::auth_get_status_internal(&auth_provider, &state.copilot_auth_state)
            .await
            .map_err(|e| ApiError::internal(format!("failed to load auth status: {e}")))?;
    Ok(Json(status))
}

async fn auth_remove_account(
    State(state): State<WebApiState>,
    Json(payload): Json<AuthAccountRequest>,
) -> Result<StatusCode, ApiError> {
    crate::commands::auth_remove_account_internal(
        &payload.auth_provider,
        &payload.account_id,
        &state.copilot_auth_state,
    )
    .await
    .map_err(|e| ApiError::internal(format!("failed to remove auth account: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn auth_set_default_account(
    State(state): State<WebApiState>,
    Json(payload): Json<AuthAccountRequest>,
) -> Result<StatusCode, ApiError> {
    crate::commands::auth_set_default_account_internal(
        &payload.auth_provider,
        &payload.account_id,
        &state.copilot_auth_state,
    )
    .await
    .map_err(|e| ApiError::internal(format!("failed to set default auth account: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn auth_logout(
    State(state): State<WebApiState>,
    Json(payload): Json<AuthStartLoginRequest>,
) -> Result<StatusCode, ApiError> {
    crate::commands::auth_logout_internal(&payload.auth_provider, &state.copilot_auth_state)
        .await
        .map_err(|e| ApiError::internal(format!("failed to logout auth provider: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_copilot_token(State(state): State<WebApiState>) -> Result<Json<String>, ApiError> {
    let auth_manager = state.copilot_auth_state.read().await;
    let token = auth_manager
        .get_valid_token()
        .await
        .map_err(|e| ApiError::internal(format!("failed to load copilot token: {e}")))?;
    Ok(Json(token))
}

async fn get_copilot_token_for_account(
    State(state): State<WebApiState>,
    Path(account_id): Path<String>,
) -> Result<Json<String>, ApiError> {
    let auth_manager = state.copilot_auth_state.read().await;
    let token = auth_manager
        .get_valid_token_for_account(&account_id)
        .await
        .map_err(|e| {
            ApiError::internal(format!(
                "failed to load copilot token for account {account_id}: {e}"
            ))
        })?;
    Ok(Json(token))
}

async fn get_copilot_models(
    State(state): State<WebApiState>,
) -> Result<Json<Vec<crate::proxy::providers::copilot_auth::CopilotModel>>, ApiError> {
    let auth_manager = state.copilot_auth_state.read().await;
    let models = auth_manager
        .fetch_models()
        .await
        .map_err(|e| ApiError::internal(format!("failed to load copilot models: {e}")))?;
    Ok(Json(models))
}

async fn get_copilot_models_for_account(
    State(state): State<WebApiState>,
    Path(account_id): Path<String>,
) -> Result<Json<Vec<crate::proxy::providers::copilot_auth::CopilotModel>>, ApiError> {
    let auth_manager = state.copilot_auth_state.read().await;
    let models = auth_manager
        .fetch_models_for_account(&account_id)
        .await
        .map_err(|e| {
            ApiError::internal(format!(
                "failed to load copilot models for account {account_id}: {e}"
            ))
        })?;
    Ok(Json(models))
}

async fn get_copilot_usage(
    State(state): State<WebApiState>,
) -> Result<Json<crate::proxy::providers::copilot_auth::CopilotUsageResponse>, ApiError> {
    let auth_manager = state.copilot_auth_state.read().await;
    let usage = auth_manager
        .fetch_usage()
        .await
        .map_err(|e| ApiError::internal(format!("failed to load copilot usage: {e}")))?;
    Ok(Json(usage))
}

async fn get_copilot_usage_for_account(
    State(state): State<WebApiState>,
    Path(account_id): Path<String>,
) -> Result<Json<crate::proxy::providers::copilot_auth::CopilotUsageResponse>, ApiError> {
    let auth_manager = state.copilot_auth_state.read().await;
    let usage = auth_manager
        .fetch_usage_for_account(&account_id)
        .await
        .map_err(|e| {
            ApiError::internal(format!(
                "failed to load copilot usage for account {account_id}: {e}"
            ))
        })?;
    Ok(Json(usage))
}

async fn create_db_backup(State(state): State<WebApiState>) -> Result<Json<String>, ApiError> {
    let db = state.app_state.db.clone();
    let filename = tokio::task::spawn_blocking(move || match db.backup_database_file()? {
        Some(path) => Ok(path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()),
        None => Err(crate::error::AppError::Config(
            "Database file not found, backup skipped".to_string(),
        )),
    })
    .await
    .map_err(|e| ApiError::internal(format!("failed to create database backup: {e}")))?
    .map_err(|e| ApiError::internal(format!("failed to create database backup: {e}")))?;

    Ok(Json(filename))
}

async fn list_db_backups() -> Result<Json<Vec<crate::database::backup::BackupEntry>>, ApiError> {
    let backups = crate::Database::list_backups()
        .map_err(|e| ApiError::internal(format!("failed to list database backups: {e}")))?;
    Ok(Json(backups))
}

async fn restore_db_backup(
    State(state): State<WebApiState>,
    Path(filename): Path<String>,
) -> Result<Json<String>, ApiError> {
    let db = state.app_state.db.clone();
    let safety_backup_id = tokio::task::spawn_blocking(move || db.restore_from_backup(&filename))
        .await
        .map_err(|e| ApiError::internal(format!("failed to restore database backup: {e}")))?
        .map_err(|e| ApiError::internal(format!("failed to restore database backup: {e}")))?;
    Ok(Json(safety_backup_id))
}

async fn rename_db_backup(
    Json(payload): Json<RenameBackupRequest>,
) -> Result<Json<String>, ApiError> {
    let filename = crate::Database::rename_backup(&payload.old_filename, &payload.new_name)
        .map_err(|e| ApiError::internal(format!("failed to rename database backup: {e}")))?;
    Ok(Json(filename))
}

async fn delete_db_backup(Path(filename): Path<String>) -> Result<StatusCode, ApiError> {
    crate::Database::delete_backup(&filename)
        .map_err(|e| ApiError::internal(format!("failed to delete database backup: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_providers(
    State(state): State<WebApiState>,
    Path(app): Path<String>,
) -> Result<Json<ProvidersResponse>, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    let providers =
        ProviderService::list(state.app_state.as_ref(), app_type.clone()).map_err(|e| {
            ApiError::internal(format!(
                "failed to load providers for {}: {e}",
                app_type.as_str()
            ))
        })?;
    let current_provider_id = ProviderService::current(state.app_state.as_ref(), app_type.clone())
        .map_err(|e| {
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
    let current_provider_id = ProviderService::current(state.app_state.as_ref(), app_type.clone())
        .map_err(|e| {
            ApiError::internal(format!(
                "failed to load current provider for {}: {e}",
                app_type.as_str()
            ))
        })?;

    Ok(Json(current_provider_id))
}

async fn get_config_dir(Path(app): Path<String>) -> Result<Json<String>, ApiError> {
    let dir = crate::commands::get_config_dir(app)
        .await
        .map_err(|e| ApiError::internal(format!("failed to load config dir: {e}")))?;
    Ok(Json(dir))
}

async fn get_omo_local_file() -> Result<Json<OmoLocalFileData>, ApiError> {
    let data = OmoService::read_local_file(&STANDARD)
        .map_err(|e| ApiError::internal(format!("failed to read OMO local file: {e}")))?;
    Ok(Json(data))
}

async fn get_current_omo_provider_id(
    State(state): State<WebApiState>,
) -> Result<Json<String>, ApiError> {
    let provider = state
        .app_state
        .db
        .get_current_omo_provider("opencode", "omo")
        .map_err(|e| ApiError::internal(format!("failed to load current OMO provider: {e}")))?;
    Ok(Json(provider.map(|p| p.id).unwrap_or_default()))
}

async fn disable_current_omo(State(state): State<WebApiState>) -> Result<StatusCode, ApiError> {
    let providers = state
        .app_state
        .db
        .get_all_providers("opencode")
        .map_err(|e| ApiError::internal(format!("failed to load OMO providers: {e}")))?;
    for (id, provider) in &providers {
        if provider.category.as_deref() == Some("omo") {
            state
                .app_state
                .db
                .clear_omo_provider_current("opencode", id, "omo")
                .map_err(|e| {
                    ApiError::internal(format!("failed to clear current OMO provider: {e}"))
                })?;
        }
    }
    OmoService::delete_config_file(&STANDARD)
        .map_err(|e| ApiError::internal(format!("failed to delete OMO config file: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_omo_slim_local_file() -> Result<Json<OmoLocalFileData>, ApiError> {
    let data = OmoService::read_local_file(&SLIM)
        .map_err(|e| ApiError::internal(format!("failed to read OMO Slim local file: {e}")))?;
    Ok(Json(data))
}

async fn get_current_omo_slim_provider_id(
    State(state): State<WebApiState>,
) -> Result<Json<String>, ApiError> {
    let provider = state
        .app_state
        .db
        .get_current_omo_provider("opencode", "omo-slim")
        .map_err(|e| {
            ApiError::internal(format!("failed to load current OMO Slim provider: {e}"))
        })?;
    Ok(Json(provider.map(|p| p.id).unwrap_or_default()))
}

async fn disable_current_omo_slim(
    State(state): State<WebApiState>,
) -> Result<StatusCode, ApiError> {
    let providers = state
        .app_state
        .db
        .get_all_providers("opencode")
        .map_err(|e| ApiError::internal(format!("failed to load OMO Slim providers: {e}")))?;
    for (id, provider) in &providers {
        if provider.category.as_deref() == Some("omo-slim") {
            state
                .app_state
                .db
                .clear_omo_provider_current("opencode", id, "omo-slim")
                .map_err(|e| {
                    ApiError::internal(format!("failed to clear current OMO Slim provider: {e}"))
                })?;
        }
    }
    OmoService::delete_config_file(&SLIM)
        .map_err(|e| ApiError::internal(format!("failed to delete OMO Slim config file: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
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

async fn update_providers_sort_order(
    State(state): State<WebApiState>,
    Path(app): Path<String>,
    Json(updates): Json<Vec<crate::services::ProviderSortUpdate>>,
) -> Result<Json<bool>, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    let updated = ProviderService::update_sort_order(state.app_state.as_ref(), app_type, updates)
        .map_err(|e| {
        ApiError::internal(format!("failed to update provider sort order: {e}"))
    })?;
    Ok(Json(updated))
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

async fn is_live_takeover_active(State(state): State<WebApiState>) -> Result<Json<bool>, ApiError> {
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

async fn get_global_proxy_url(
    State(state): State<WebApiState>,
) -> Result<Json<Option<String>>, ApiError> {
    let url = state
        .app_state
        .db
        .get_global_proxy_url()
        .map_err(|e| ApiError::internal(format!("failed to load global proxy url: {e}")))?;
    Ok(Json(url))
}

async fn set_global_proxy_url(
    State(state): State<WebApiState>,
    Json(payload): Json<ValueRequest>,
) -> Result<StatusCode, ApiError> {
    let url_opt = if payload.value.trim().is_empty() {
        None
    } else {
        Some(payload.value.as_str())
    };

    http_client::validate_proxy(url_opt).map_err(ApiError::bad_request)?;
    state
        .app_state
        .db
        .set_global_proxy_url(url_opt)
        .map_err(|e| ApiError::internal(format!("failed to save global proxy url: {e}")))?;
    http_client::apply_proxy(url_opt).map_err(ApiError::bad_request)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn test_proxy_url(
    Json(payload): Json<ValueRequest>,
) -> Result<Json<crate::ProxyTestResult>, ApiError> {
    let result = crate::test_proxy_url(payload.value).await.map_err(ApiError::bad_request)?;
    Ok(Json(result))
}

async fn get_upstream_proxy_status() -> Json<crate::UpstreamProxyStatus> {
    Json(crate::get_upstream_proxy_status())
}

async fn scan_local_proxies() -> Json<Vec<crate::DetectedProxy>> {
    Json(crate::scan_local_proxies().await)
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
        .map_err(|e| {
            ApiError::internal(format!("failed to update default cost multiplier: {e}"))
        })?;
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

async fn reset_circuit_breaker(
    State(state): State<WebApiState>,
    Path((app, provider_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let app_type = AppType::from_str(&app).map_err(|e| ApiError::bad_request(e.to_string()))?;
    let app_type_str = app_type.as_str().to_string();

    state
        .app_state
        .db
        .update_provider_health(&provider_id, &app_type_str, true, None)
        .await
        .map_err(|e| ApiError::internal(format!("failed to reset provider health: {e}")))?;

    state
        .app_state
        .proxy_service
        .reset_provider_circuit_breaker(&provider_id, &app_type_str)
        .await
        .map_err(|e| {
            ApiError::internal(format!("failed to reset in-memory circuit breaker: {e}"))
        })?;

    let (app_enabled, auto_failover_enabled) = match state
        .app_state
        .db
        .get_proxy_config_for_app(&app_type_str)
        .await
    {
        Ok(config) => (config.enabled, config.auto_failover_enabled),
        Err(e) => {
            log::error!(
                "[{app_type_str}] Failed to read proxy_config: {e}, defaulting to disabled"
            );
            (false, false)
        }
    };

    if app_enabled && auto_failover_enabled && state.app_state.proxy_service.is_running().await {
        let current_id = state
            .app_state
            .db
            .get_current_provider(&app_type_str)
            .map_err(|e| ApiError::internal(format!("failed to load current provider: {e}")))?;

        if let Some(current_id) = current_id {
            let queue = state
                .app_state
                .db
                .get_failover_queue(&app_type_str)
                .map_err(|e| ApiError::internal(format!("failed to load failover queue: {e}")))?;

            let restored_order = queue
                .iter()
                .find(|item| item.provider_id == provider_id)
                .and_then(|item| item.sort_index);

            let current_order = queue
                .iter()
                .find(|item| item.provider_id == current_id)
                .and_then(|item| item.sort_index);

            if let (Some(restored), Some(current)) = (restored_order, current_order) {
                if restored < current {
                    state
                        .app_state
                        .proxy_service
                        .switch_proxy_target(&app_type_str, &provider_id)
                        .await
                        .map_err(|e| {
                            ApiError::internal(format!("failed to switch recovered provider: {e}"))
                        })?;
                }
            }
        }
    }

    Ok(StatusCode::NO_CONTENT)
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
        .map_err(|e| ApiError::internal(format!("failed to update circuit breaker config: {e}")))?;

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
        .map_err(|e| {
            ApiError::internal(format!("failed to add provider to failover queue: {e}"))
        })?;
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
            ApiError::internal(format!(
                "failed to remove provider from failover queue: {e}"
            ))
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
            let current_id = crate::settings::get_effective_current_provider(
                state.app_state.db.as_ref(),
                &app_type,
            )
            .map_err(|e| {
                ApiError::internal(format!("failed to get current provider for failover: {e}"))
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
    if let Err(err) = db.periodic_backup_if_needed() {
        log::warn!("startup periodic maintenance failed: {err}");
    }
    let app_state = Arc::new(AppState::new(db));
    crate::services::webdav_auto_sync::start_worker(app_state.db.clone());
    let state = WebApiState {
        copilot_auth_state: app_state.copilot_auth_state.clone(),
        app_state,
    };
    let bind_addr = resolve_bind_addr()?;

    let mut app = Router::new()
        .route("/", get(root))
        .route("/api/health", get(health))
        .route("/api/config/export", get(export_config_download))
        .route("/api/config/import", post(import_config_upload))
        .route("/api/settings", get(get_settings).put(save_settings))
        .route(
            "/api/settings/app-config-dir-override",
            get(get_app_config_dir_override).put(set_app_config_dir_override),
        )
        .route(
            "/api/settings/common-config/:app",
            get(get_common_config_snippet).put(set_common_config_snippet),
        )
        .route(
            "/api/settings/common-config/:app/extract",
            post(extract_common_config_snippet),
        )
        .route(
            "/api/settings/sync-current-providers-live",
            post(sync_current_providers_live),
        )
        .route("/api/webdav/test", post(webdav_test_connection))
        .route("/api/webdav/upload", post(webdav_sync_upload))
        .route("/api/webdav/download", post(webdav_sync_download))
        .route("/api/webdav/settings", post(webdav_sync_save_settings))
        .route(
            "/api/webdav/remote-info",
            get(webdav_sync_fetch_remote_info),
        )
        .route(
            "/api/settings/rectifier",
            get(get_rectifier_config).put(set_rectifier_config),
        )
        .route(
            "/api/settings/optimizer",
            get(get_optimizer_config).put(set_optimizer_config),
        )
        .route(
            "/api/settings/log-config",
            get(get_log_config).put(set_log_config),
        )
        .route(
            "/api/settings/stream-check-config",
            get(get_stream_check_config).put(set_stream_check_config),
        )
        .route("/api/settings/tool-versions", post(get_tool_versions))
        .route("/api/settings/config-dir/:app", get(get_config_dir))
        .route("/api/omo/local-file", get(get_omo_local_file))
        .route(
            "/api/omo/current-provider-id",
            get(get_current_omo_provider_id),
        )
        .route("/api/omo/disable", post(disable_current_omo))
        .route("/api/omo-slim/local-file", get(get_omo_slim_local_file))
        .route(
            "/api/omo-slim/current-provider-id",
            get(get_current_omo_slim_provider_id),
        )
        .route("/api/omo-slim/disable", post(disable_current_omo_slim))
        .route("/api/auth/start-login", post(auth_start_login))
        .route("/api/auth/poll-for-account", post(auth_poll_for_account))
        .route("/api/auth/:auth_provider/accounts", get(auth_list_accounts))
        .route("/api/auth/:auth_provider/status", get(auth_get_status))
        .route("/api/auth/remove-account", post(auth_remove_account))
        .route(
            "/api/auth/set-default-account",
            post(auth_set_default_account),
        )
        .route("/api/auth/logout", post(auth_logout))
        .route("/api/copilot/token", get(get_copilot_token))
        .route(
            "/api/copilot/accounts/:account_id/token",
            get(get_copilot_token_for_account),
        )
        .route("/api/copilot/models", get(get_copilot_models))
        .route(
            "/api/copilot/accounts/:account_id/models",
            get(get_copilot_models_for_account),
        )
        .route("/api/copilot/usage", get(get_copilot_usage))
        .route(
            "/api/copilot/accounts/:account_id/usage",
            get(get_copilot_usage_for_account),
        )
        .route(
            "/api/backups/db",
            get(list_db_backups).post(create_db_backup),
        )
        .route("/api/backups/db/rename", put(rename_db_backup))
        .route(
            "/api/backups/db/:filename",
            axum::routing::delete(delete_db_backup),
        )
        .route("/api/backups/db/:filename/restore", post(restore_db_backup))
        .route("/api/providers/:app", get(get_providers).post(add_provider))
        .route(
            "/api/providers/:app/sort-order",
            put(update_providers_sort_order),
        )
        .route("/api/providers/:app/current", get(get_current_provider))
        .route(
            "/api/providers/:app/import-default",
            post(import_default_provider_config),
        )
        .route(
            "/api/providers/:app/stream-check/:id",
            post(stream_check_provider),
        )
        .route(
            "/api/providers/:app/stream-check-all",
            post(stream_check_all_providers),
        )
        .route(
            "/api/providers/:app/live-settings",
            get(read_live_provider_settings),
        )
        .route("/api/providers/:app/:id/usage", get(query_provider_usage))
        .route(
            "/api/providers/:app/:id/usage/test",
            post(test_usage_script),
        )
        .route("/api/providers/endpoints/test", post(test_api_endpoints))
        .route(
            "/api/providers/:app/:id/custom-endpoints",
            get(get_custom_endpoints)
                .post(add_custom_endpoint)
                .delete(remove_custom_endpoint),
        )
        .route(
            "/api/providers/:app/:id/custom-endpoints/last-used",
            post(update_endpoint_last_used),
        )
        .route(
            "/api/providers/:app/live-provider-ids",
            get(get_live_provider_ids),
        )
        .route(
            "/api/providers/:app/import-live",
            post(import_providers_from_live),
        )
        .route(
            "/api/providers/:app/live-config/:id",
            axum::routing::delete(remove_provider_from_live_config),
        )
        .route(
            "/api/universal-providers",
            get(get_universal_providers).post(upsert_universal_provider),
        )
        .route(
            "/api/universal-providers/:id",
            get(get_universal_provider).delete(delete_universal_provider),
        )
        .route(
            "/api/universal-providers/:id/sync",
            post(sync_universal_provider),
        )
        .route("/api/skills/installed", get(get_installed_skills))
        .route("/api/skills/backups", get(get_skill_backups))
        .route("/api/skills/unmanaged", get(scan_unmanaged_skills))
        .route("/api/skills/import", post(import_skills_from_apps))
        .route(
            "/api/skills/repos",
            get(get_skill_repos).post(add_skill_repo),
        )
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
        .route(
            "/api/openclaw/env",
            get(get_openclaw_env).put(set_openclaw_env),
        )
        .route(
            "/api/openclaw/tools",
            get(get_openclaw_tools).put(set_openclaw_tools),
        )
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
        .route(
            "/api/mcp/servers",
            get(get_mcp_servers).post(upsert_mcp_server),
        )
        .route("/api/mcp/servers/import", post(import_mcp_from_apps))
        .route(
            "/api/mcp/servers/:id",
            axum::routing::delete(delete_mcp_server),
        )
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
            "/api/proxy/global-url",
            get(get_global_proxy_url).put(set_global_proxy_url),
        )
        .route("/api/proxy/test", post(test_proxy_url))
        .route("/api/proxy/upstream-status", get(get_upstream_proxy_status))
        .route("/api/proxy/scan-local", get(scan_local_proxies))
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
        .route(
            "/api/proxy/stop-with-restore",
            post(stop_proxy_with_restore),
        )
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
            "/api/failover/apps/:app/providers/:provider_id/reset-circuit-breaker",
            post(reset_circuit_breaker),
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
