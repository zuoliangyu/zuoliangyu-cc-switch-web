#![allow(non_snake_case)]

use indexmap::IndexMap;

use tauri::State;

use crate::app_config::AppType;
use crate::services::McpService;
use crate::store::AppState;
use std::str::FromStr;

// ============================================================================
// v3.7.0 新增：统一 MCP 管理命令
// ============================================================================

use crate::app_config::McpServer;

pub(crate) async fn get_mcp_servers_internal(
    state: &AppState,
) -> Result<IndexMap<String, McpServer>, String> {
    McpService::get_all_servers(state).map_err(|e| e.to_string())
}

/// 获取所有 MCP 服务器（统一结构）
#[tauri::command]
pub async fn get_mcp_servers(
    state: State<'_, AppState>,
) -> Result<IndexMap<String, McpServer>, String> {
    get_mcp_servers_internal(state.inner()).await
}

/// 添加或更新 MCP 服务器
pub(crate) async fn upsert_mcp_server_internal(
    state: &AppState,
    server: McpServer,
) -> Result<(), String> {
    McpService::upsert_server(state, server).map_err(|e| e.to_string())
}

/// 添加或更新 MCP 服务器
#[tauri::command]
pub async fn upsert_mcp_server(
    state: State<'_, AppState>,
    server: McpServer,
) -> Result<(), String> {
    upsert_mcp_server_internal(state.inner(), server).await
}

/// 删除 MCP 服务器
pub(crate) async fn delete_mcp_server_internal(
    state: &AppState,
    id: String,
) -> Result<bool, String> {
    McpService::delete_server(state, &id).map_err(|e| e.to_string())
}

/// 删除 MCP 服务器
#[tauri::command]
pub async fn delete_mcp_server(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    delete_mcp_server_internal(state.inner(), id).await
}

/// 切换 MCP 服务器在指定应用的启用状态
pub(crate) async fn toggle_mcp_app_internal(
    state: &AppState,
    server_id: String,
    app: String,
    enabled: bool,
) -> Result<(), String> {
    let app_ty = AppType::from_str(&app).map_err(|e| e.to_string())?;
    McpService::toggle_app(state, &server_id, app_ty, enabled).map_err(|e| e.to_string())
}

/// 切换 MCP 服务器在指定应用的启用状态
#[tauri::command]
pub async fn toggle_mcp_app(
    state: State<'_, AppState>,
    server_id: String,
    app: String,
    enabled: bool,
) -> Result<(), String> {
    toggle_mcp_app_internal(state.inner(), server_id, app, enabled).await
}

/// 从所有应用导入 MCP 服务器（复用已有的导入逻辑）
pub(crate) async fn import_mcp_from_apps_internal(state: &AppState) -> Result<usize, String> {
    let mut total = 0;
    total += McpService::import_from_claude(state).unwrap_or(0);
    total += McpService::import_from_codex(state).unwrap_or(0);
    total += McpService::import_from_gemini(state).unwrap_or(0);
    total += McpService::import_from_opencode(state).unwrap_or(0);
    Ok(total)
}

/// 从所有应用导入 MCP 服务器（复用已有的导入逻辑）
#[tauri::command]
pub async fn import_mcp_from_apps(state: State<'_, AppState>) -> Result<usize, String> {
    import_mcp_from_apps_internal(state.inner()).await
}
