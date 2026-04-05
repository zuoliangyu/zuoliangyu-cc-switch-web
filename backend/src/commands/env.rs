use crate::services::env_checker::EnvConflict;
use crate::services::env_manager::BackupInfo;

pub(crate) fn check_env_conflicts_internal(app: String) -> Result<Vec<EnvConflict>, String> {
    crate::services::env_checker::check_env_conflicts(&app)
}

pub(crate) fn delete_env_vars_internal(conflicts: Vec<EnvConflict>) -> Result<BackupInfo, String> {
    crate::services::env_manager::delete_env_vars(conflicts)
}

pub(crate) fn restore_env_backup_internal(backup_path: String) -> Result<(), String> {
    crate::services::env_manager::restore_from_backup(backup_path)
}
