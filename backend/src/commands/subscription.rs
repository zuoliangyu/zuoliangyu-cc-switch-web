use crate::services::subscription::SubscriptionQuota;

pub(crate) async fn get_subscription_quota_internal(
    tool: String,
) -> Result<SubscriptionQuota, String> {
    crate::services::subscription::get_subscription_quota(&tool).await
}
