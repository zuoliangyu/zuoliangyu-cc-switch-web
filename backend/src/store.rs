use crate::database::Database;
use crate::proxy::providers::copilot_auth::CopilotAuthManager;
use crate::services::proxy::ProxyService;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 全局应用状态
pub struct AppState {
    pub db: Arc<Database>,
    pub copilot_auth_state: Arc<RwLock<CopilotAuthManager>>,
    pub proxy_service: ProxyService,
}

impl AppState {
    /// 创建新的应用状态
    pub fn new(db: Arc<Database>) -> Self {
        let copilot_auth_state = Arc::new(RwLock::new(CopilotAuthManager::new(
            crate::config::get_app_config_dir(),
        )));
        let proxy_service = ProxyService::new_with_auth(db.clone(), copilot_auth_state.clone());

        Self {
            db,
            copilot_auth_state,
            proxy_service,
        }
    }
}
