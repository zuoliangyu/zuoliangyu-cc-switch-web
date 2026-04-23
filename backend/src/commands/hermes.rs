pub(crate) fn get_hermes_memory_internal(
    kind: crate::hermes_config::MemoryKind,
) -> Result<String, String> {
    crate::hermes_config::read_memory(kind).map_err(|e| e.to_string())
}

pub(crate) fn scan_hermes_config_health_internal(
) -> Result<Vec<crate::hermes_config::HermesHealthWarning>, String> {
    crate::hermes_config::scan_hermes_config_health().map_err(|e| e.to_string())
}

pub(crate) fn set_hermes_memory_internal(
    kind: crate::hermes_config::MemoryKind,
    content: String,
) -> Result<(), String> {
    crate::hermes_config::write_memory(kind, &content).map_err(|e| e.to_string())
}

pub(crate) fn get_hermes_memory_limits_internal(
) -> Result<crate::hermes_config::HermesMemoryLimits, String> {
    crate::hermes_config::read_memory_limits().map_err(|e| e.to_string())
}

pub(crate) fn set_hermes_memory_enabled_internal(
    kind: crate::hermes_config::MemoryKind,
    enabled: bool,
) -> Result<(), String> {
    crate::hermes_config::set_memory_enabled(kind, enabled).map_err(|e| e.to_string())
}
