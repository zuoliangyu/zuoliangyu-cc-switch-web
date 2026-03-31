use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, get_service, post, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

use crate::app_config::AppType;
use crate::database::FailoverQueueItem;
use crate::provider::Provider;
use crate::proxy::circuit_breaker::{CircuitBreakerConfig, CircuitBreakerStats};
use crate::proxy::types::{
    AppProxyConfig, GlobalProxyConfig, ProviderHealth, ProxyConfig, ProxyServerInfo,
    ProxyStatus, ProxyTakeoverStatus,
};
use crate::services::{ProviderService, SwitchResult};
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
        .route("/api/providers/:app", get(get_providers).post(add_provider))
        .route("/api/providers/:app/current", get(get_current_provider))
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
