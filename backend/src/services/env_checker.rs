use serde::{Deserialize, Serialize};

#[cfg(not(target_os = "windows"))]
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvConflict {
    pub var_name: String,
    pub var_value: String,
    pub source_type: String,
    pub source_path: String,
}

#[cfg(target_os = "windows")]
use winreg::enums::*;
#[cfg(target_os = "windows")]
use winreg::RegKey;

pub fn check_env_conflicts(app: &str) -> Result<Vec<EnvConflict>, String> {
    let keywords = match app.to_lowercase().as_str() {
        "claude" => vec!["ANTHROPIC"],
        "codex" => vec!["OPENAI"],
        "gemini" => vec!["GEMINI", "GOOGLE_GEMINI"],
        _ => vec![],
    };

    #[cfg(target_os = "windows")]
    let conflicts = check_system_env(&keywords)?;

    #[cfg(not(target_os = "windows"))]
    let mut conflicts = check_system_env(&keywords)?;

    #[cfg(not(target_os = "windows"))]
    {
        conflicts.extend(check_shell_configs(&keywords)?);
    }

    Ok(conflicts)
}

#[cfg(target_os = "windows")]
fn check_system_env(keywords: &[&str]) -> Result<Vec<EnvConflict>, String> {
    let mut conflicts = Vec::new();

    if let Ok(hkcu) = RegKey::predef(HKEY_CURRENT_USER).open_subkey("Environment") {
        for (name, value) in hkcu.enum_values().filter_map(Result::ok) {
            if keywords.iter().any(|keyword| name.to_uppercase().contains(keyword)) {
                conflicts.push(EnvConflict {
                    var_name: name.clone(),
                    var_value: value.to_string(),
                    source_type: "system".to_string(),
                    source_path: "HKEY_CURRENT_USER\\Environment".to_string(),
                });
            }
        }
    }

    if let Ok(hklm) = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey("SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment")
    {
        for (name, value) in hklm.enum_values().filter_map(Result::ok) {
            if keywords.iter().any(|keyword| name.to_uppercase().contains(keyword)) {
                conflicts.push(EnvConflict {
                    var_name: name.clone(),
                    var_value: value.to_string(),
                    source_type: "system".to_string(),
                    source_path:
                        "HKEY_LOCAL_MACHINE\\SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment"
                            .to_string(),
                });
            }
        }
    }

    Ok(conflicts)
}

#[cfg(not(target_os = "windows"))]
fn check_system_env(keywords: &[&str]) -> Result<Vec<EnvConflict>, String> {
    Ok(std::env::vars()
        .filter(|(key, _)| keywords.iter().any(|keyword| key.to_uppercase().contains(keyword)))
        .map(|(key, value)| EnvConflict {
            var_name: key,
            var_value: value,
            source_type: "system".to_string(),
            source_path: "Process Environment".to_string(),
        })
        .collect())
}

#[cfg(not(target_os = "windows"))]
fn check_shell_configs(keywords: &[&str]) -> Result<Vec<EnvConflict>, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let config_files = vec![
        format!("{home}/.bashrc"),
        format!("{home}/.bash_profile"),
        format!("{home}/.zshrc"),
        format!("{home}/.zprofile"),
        format!("{home}/.profile"),
        "/etc/profile".to_string(),
        "/etc/bashrc".to_string(),
    ];

    let mut conflicts = Vec::new();
    for file_path in config_files {
        if let Ok(content) = fs::read_to_string(&file_path) {
            for (index, line) in content.lines().enumerate() {
                let trimmed = line.trim();
                if trimmed.starts_with("export ")
                    || (!trimmed.starts_with('#') && trimmed.contains('='))
                {
                    let export_line = trimmed.strip_prefix("export ").unwrap_or(trimmed);
                    if let Some(eq_pos) = export_line.find('=') {
                        let var_name = export_line[..eq_pos].trim();
                        if keywords
                            .iter()
                            .any(|keyword| var_name.to_uppercase().contains(keyword))
                        {
                            conflicts.push(EnvConflict {
                                var_name: var_name.to_string(),
                                var_value: export_line[eq_pos + 1..]
                                    .trim()
                                    .trim_matches('"')
                                    .trim_matches('\'')
                                    .to_string(),
                                source_type: "file".to_string(),
                                source_path: format!("{file_path}:{}", index + 1),
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(conflicts)
}
