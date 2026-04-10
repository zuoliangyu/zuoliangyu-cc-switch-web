use crate::proxy::providers::codex_oauth_auth::CodexOAuthManager;
use crate::services::subscription::{query_codex_quota, CredentialStatus, SubscriptionQuota};
use std::sync::Arc;
use tokio::sync::RwLock;

pub(crate) async fn get_codex_oauth_quota_internal(
    account_id: Option<String>,
    state: &Arc<RwLock<CodexOAuthManager>>,
) -> Result<SubscriptionQuota, String> {
    let manager = state.read().await;

    let resolved = match account_id {
        Some(id) => Some(id),
        None => manager.default_account_id().await,
    };
    let Some(id) = resolved else {
        return Ok(SubscriptionQuota::not_found("codex_oauth"));
    };

    let token = match manager.get_valid_token_for_account(&id).await {
        Ok(token) => token,
        Err(error) => {
            return Ok(SubscriptionQuota::error(
                "codex_oauth",
                CredentialStatus::Expired,
                format!("Codex OAuth token unavailable: {error}"),
            ));
        }
    };

    Ok(query_codex_quota(
        &token,
        Some(&id),
        "codex_oauth",
        "Codex OAuth access token expired or rejected. Please re-login via cc-switch.",
    )
    .await)
}
