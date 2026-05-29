use super::CmdResult;
use smartstring::SmartString;

const SERVICE_DISABLED: &str = "本版本已移除系统服务模式，内核仅以 Sidecar 运行。";

#[tauri::command]
pub async fn install_service() -> CmdResult {
    Err(SmartString::from(SERVICE_DISABLED))
}

#[tauri::command]
pub async fn uninstall_service() -> CmdResult {
    Err(SmartString::from(SERVICE_DISABLED))
}

#[tauri::command]
pub async fn reinstall_service() -> CmdResult {
    Err(SmartString::from(SERVICE_DISABLED))
}

#[tauri::command]
pub async fn repair_service() -> CmdResult {
    Err(SmartString::from(SERVICE_DISABLED))
}

#[tauri::command]
pub async fn is_service_available() -> CmdResult<bool> {
    Ok(false)
}
