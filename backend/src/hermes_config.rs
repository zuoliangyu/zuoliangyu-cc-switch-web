use crate::config::{atomic_write, get_home_dir};
use crate::error::AppError;
use crate::settings::get_hermes_override_dir;
use serde::{Deserialize, Serialize};
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
