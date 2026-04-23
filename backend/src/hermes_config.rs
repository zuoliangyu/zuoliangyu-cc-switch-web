use crate::config::{atomic_write, get_home_dir};
use crate::error::AppError;
use crate::settings::get_hermes_override_dir;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub fn get_hermes_dir() -> PathBuf {
    if let Some(override_dir) = get_hermes_override_dir() {
        return override_dir;
    }

    get_default_hermes_dir()
}

pub fn get_default_hermes_dir() -> PathBuf {
    get_home_dir().join(".hermes")
}

fn get_hermes_config_path() -> PathBuf {
    get_hermes_dir().join("config.yaml")
}

fn memories_dir() -> PathBuf {
    get_hermes_dir().join("memories")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryKind {
    Memory,
    User,
}

impl MemoryKind {
    fn filename(self) -> &'static str {
        match self {
            Self::Memory => "MEMORY.md",
            Self::User => "USER.md",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HermesMemoryLimits {
    pub memory: usize,
    pub user: usize,
    pub memory_enabled: bool,
    pub user_enabled: bool,
}

impl Default for HermesMemoryLimits {
    fn default() -> Self {
        Self {
            memory: 2200,
            user: 1375,
            memory_enabled: true,
            user_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HermesHealthWarning {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

fn read_hermes_config() -> Result<serde_yaml::Value, AppError> {
    let path = get_hermes_config_path();
    if !path.exists() {
        return Ok(serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
    }

    let content = fs::read_to_string(&path).map_err(|e| AppError::io(&path, e))?;
    if content.trim().is_empty() {
        return Ok(serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
    }

    serde_yaml::from_str(&content)
        .map_err(|e| AppError::Config(format!("Failed to parse Hermes config as YAML: {e}")))
}

pub fn scan_hermes_config_health() -> Result<Vec<HermesHealthWarning>, AppError> {
    let path = get_hermes_config_path();
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&path).map_err(|e| AppError::io(&path, e))?;
    Ok(scan_hermes_health_internal(&content))
}

fn scan_hermes_health_internal(content: &str) -> Vec<HermesHealthWarning> {
    let mut warnings = Vec::new();

    if content.trim().is_empty() {
        return warnings;
    }

    let config = match serde_yaml::from_str::<serde_yaml::Value>(content) {
        Ok(value) => value,
        Err(error) => {
            warnings.push(hermes_warning(
                "config_parse_failed",
                format!("Hermes config could not be parsed as YAML: {error}"),
                Some(get_hermes_config_path().display().to_string()),
            ));
            return warnings;
        }
    };

    if let Some(model) = config.get("model") {
        if model.get("default").is_none() && model.get("provider").is_none() {
            warnings.push(hermes_warning(
                "model_no_default",
                "No default model or provider configured in 'model' section".to_string(),
                Some("model".to_string()),
            ));
        }
    }

    if config
        .get("custom_providers")
        .and_then(|value| value.as_mapping())
        .is_some()
    {
        warnings.push(hermes_warning(
            "custom_providers_not_list",
            "custom_providers should be a YAML list (sequence), not a mapping".to_string(),
            Some("custom_providers".to_string()),
        ));
    }

    let mut provider_models: HashMap<String, Vec<String>> = HashMap::new();
    let mut name_counts: HashMap<String, usize> = HashMap::new();
    let mut base_url_counts: HashMap<String, usize> = HashMap::new();

    if let Some(sequence) = config.get("custom_providers").and_then(|value| value.as_sequence()) {
        for item in sequence {
            if let Some(name) = item.get("name").and_then(yaml_as_non_empty_str) {
                *name_counts.entry(name.to_string()).or_insert(0) += 1;
                if let Some(models) = item.get("models").and_then(|value| value.as_mapping()) {
                    provider_models
                        .entry(name.to_string())
                        .or_insert_with(|| collect_mapping_string_keys(models));
                }
            }

            if let Some(base_url) = item
                .get("base_url")
                .and_then(yaml_as_non_empty_str)
                .map(|value| value.trim_end_matches('/').to_lowercase())
                .filter(|value| !value.is_empty())
            {
                *base_url_counts.entry(base_url).or_insert(0) += 1;
            }
        }
    }

    for (name, count) in &name_counts {
        if *count > 1 {
            warnings.push(hermes_warning(
                "duplicate_provider_name",
                format!(
                    "Duplicate provider name '{name}' in custom_providers; only one entry will be used"
                ),
                Some("custom_providers".to_string()),
            ));
        }
    }

    for (base_url, count) in &base_url_counts {
        if *count > 1 {
            warnings.push(hermes_warning(
                "duplicate_provider_base_url",
                format!(
                    "Duplicate base_url '{base_url}' in custom_providers; possible accidental copy"
                ),
                Some("custom_providers".to_string()),
            ));
        }
    }

    if let Some(model) = config.get("model") {
        if let Some(provider_ref) = model.get("provider").and_then(yaml_as_non_empty_str) {
            if !name_counts.contains_key(provider_ref) {
                warnings.push(hermes_warning(
                    "model_provider_unknown",
                    format!(
                        "model.provider '{provider_ref}' does not match any configured provider"
                    ),
                    Some("model.provider".to_string()),
                ));
            } else if let Some(default_model) =
                model.get("default").and_then(yaml_as_non_empty_str)
            {
                if let Some(model_ids) = provider_models.get(provider_ref) {
                    if !model_ids.is_empty() && !model_ids.iter().any(|id| id == default_model) {
                        warnings.push(hermes_warning(
                            "model_default_not_in_provider",
                            format!(
                                "model.default '{default_model}' is not in provider '{provider_ref}' models list"
                            ),
                            Some("model.default".to_string()),
                        ));
                    }
                }
            }
        }
    }

    let version = config
        .get("_config_version")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let providers_dict_populated = config
        .get("providers")
        .and_then(|value| value.as_mapping())
        .map(|mapping| !mapping.is_empty())
        .unwrap_or(false);
    if version >= 12 && providers_dict_populated {
        warnings.push(hermes_warning(
            "schema_migrated_v12",
            "Hermes newer schema moved some entries into the 'providers' dict; CC Switch currently treats them as read-only".to_string(),
            Some("providers".to_string()),
        ));
    }

    warnings
}

fn hermes_warning(
    code: &str,
    message: String,
    path: Option<String>,
) -> HermesHealthWarning {
    HermesHealthWarning {
        code: code.to_string(),
        message,
        path,
    }
}

fn yaml_as_non_empty_str(value: &serde_yaml::Value) -> Option<&str> {
    value
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn collect_mapping_string_keys(mapping: &serde_yaml::Mapping) -> Vec<String> {
    mapping
        .keys()
        .filter_map(|key| key.as_str().map(ToString::to_string))
        .collect()
}

fn is_top_level_key_line(line: &str) -> bool {
    if line.is_empty() {
        return false;
    }
    let first_char = line.as_bytes()[0];
    if matches!(first_char, b' ' | b'\t' | b'#' | b'-') {
        return false;
    }

    if let Some(colon_pos) = line.find(':') {
        let after_colon = &line[colon_pos + 1..];
        after_colon.is_empty() || after_colon.starts_with(' ') || after_colon.starts_with('\t')
    } else {
        false
    }
}

fn find_yaml_section_range(raw: &str, section_key: &str) -> Option<(usize, usize)> {
    let target = format!("{section_key}:");
    let mut section_start = None;
    let mut offset = 0;

    for line in raw.split('\n') {
        if section_start.is_none() && is_top_level_key_line(line) && line.starts_with(&target) {
            let after_target = &line[target.len()..];
            if after_target.is_empty()
                || after_target.starts_with(' ')
                || after_target.starts_with('\t')
                || after_target.starts_with('\r')
            {
                section_start = Some(offset);
            }
        } else if section_start.is_some() && is_top_level_key_line(line) {
            return Some((section_start.unwrap(), offset));
        }

        offset += line.len() + 1;
    }

    section_start.map(|start| (start, raw.len()))
}

fn serialize_yaml_section(key: &str, value: &serde_yaml::Value) -> Result<String, AppError> {
    let mut section = serde_yaml::Mapping::new();
    section.insert(serde_yaml::Value::String(key.to_string()), value.clone());
    serde_yaml::to_string(&serde_yaml::Value::Mapping(section))
        .map_err(|e| AppError::Config(format!("Failed to serialize YAML section '{key}': {e}")))
}

fn replace_yaml_section(
    raw: &str,
    section_key: &str,
    value: &serde_yaml::Value,
) -> Result<String, AppError> {
    let serialized = serialize_yaml_section(section_key, value)?;

    if let Some((start, end)) = find_yaml_section_range(raw, section_key) {
        let mut result = String::with_capacity(raw.len());
        result.push_str(&raw[..start]);
        result.push_str(&serialized);
        let remainder = &raw[end..];
        if !serialized.ends_with('\n') && !remainder.is_empty() && !remainder.starts_with('\n') {
            result.push('\n');
        }
        result.push_str(remainder);
        Ok(result)
    } else {
        let mut result = raw.to_string();
        if !result.is_empty() && !result.ends_with('\n') {
            result.push('\n');
        }
        result.push_str(&serialized);
        if !result.ends_with('\n') {
            result.push('\n');
        }
        Ok(result)
    }
}

fn write_memory_section(memory: &serde_yaml::Mapping) -> Result<(), AppError> {
    let path = get_hermes_config_path();
    let raw = if path.exists() {
        fs::read_to_string(&path).map_err(|e| AppError::io(&path, e))?
    } else {
        String::new()
    };

    let next = replace_yaml_section(&raw, "memory", &serde_yaml::Value::Mapping(memory.clone()))?;
    atomic_write(&path, next.as_bytes())
}

pub fn read_memory(kind: MemoryKind) -> Result<String, AppError> {
    let path = memories_dir().join(kind.filename());
    match fs::read_to_string(&path) {
        Ok(content) => Ok(content),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(AppError::io(&path, error)),
    }
}

pub fn write_memory(kind: MemoryKind, content: &str) -> Result<(), AppError> {
    let path = memories_dir().join(kind.filename());
    atomic_write(&path, content.as_bytes())
}

pub fn read_memory_limits() -> Result<HermesMemoryLimits, AppError> {
    let mut limits = HermesMemoryLimits::default();
    let config = read_hermes_config()?;
    let Some(memory) = config.get("memory") else {
        return Ok(limits);
    };

    if let Some(value) = memory.get("memory_char_limit").and_then(|v| v.as_u64()) {
        limits.memory = value as usize;
    }
    if let Some(value) = memory.get("user_char_limit").and_then(|v| v.as_u64()) {
        limits.user = value as usize;
    }
    if let Some(value) = memory.get("memory_enabled").and_then(|v| v.as_bool()) {
        limits.memory_enabled = value;
    }
    if let Some(value) = memory
        .get("user_profile_enabled")
        .and_then(|v| v.as_bool())
    {
        limits.user_enabled = value;
    }

    Ok(limits)
}

pub fn set_memory_enabled(kind: MemoryKind, enabled: bool) -> Result<(), AppError> {
    let config = read_hermes_config()?;
    let mut memory = match config.get("memory") {
        Some(serde_yaml::Value::Mapping(mapping)) => mapping.clone(),
        _ => serde_yaml::Mapping::new(),
    };

    let key = match kind {
        MemoryKind::Memory => "memory_enabled",
        MemoryKind::User => "user_profile_enabled",
    };
    memory.insert(
        serde_yaml::Value::String(key.to_string()),
        serde_yaml::Value::Bool(enabled),
    );

    write_memory_section(&memory)
}
