//! 代理服务器模块
//!
//! 提供本地HTTP代理服务，支持多Provider故障转移和请求透传

pub mod body_filter;
pub mod cache_injector;
pub mod circuit_breaker;
pub mod error;
pub mod error_mapper;
pub(crate) mod failover_switch;
mod forwarder;
pub mod handler_config;
pub mod handler_context;
mod handlers;
pub mod http_client;
pub mod hyper_client;
pub mod log_codes;
pub mod model_mapper;
pub mod provider_router;
pub mod providers;
pub mod response_processor;
pub(crate) mod server;
mod session;
pub(crate) mod sse;
pub mod thinking_budget_rectifier;
pub mod thinking_optimizer;
pub mod thinking_rectifier;
pub(crate) mod types;
pub mod usage;

// 公开导出给外部使用（commands, services等模块需要）
pub use error::ProxyError;
pub(crate) use session::extract_session_id;
