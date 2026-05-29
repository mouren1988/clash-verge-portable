use crate::core::handle;
use anyhow::Result;
use async_trait::async_trait;
use clash_verge_logging::{Type, logging};
use once_cell::sync::OnceCell;
#[cfg(unix)]
use std::iter;
use std::{fs, path::PathBuf};
use tauri::Manager as _;

/// 与 `tauri.conf.json` 的 `identifier` 一致（Tauri 用于默认 AppData 路径等）。
#[cfg(not(feature = "verge-dev"))]
pub static APP_ID: &str = "io.github.clash-verge-rev.clash-verge-rev";
#[cfg(not(feature = "verge-dev"))]
pub static BACKUP_DIR: &str = "clash-verge-rev-backup";

#[cfg(feature = "verge-dev")]
pub static APP_ID: &str = "io.github.clash-verge-rev.clash-verge-rev.dev";
#[cfg(feature = "verge-dev")]
pub static BACKUP_DIR: &str = "clash-verge-rev-backup-dev";

/// 历史版本目录名（与早期便携目录布局一致），仅用于迁移；与当前 `APP_ID` 相同时跳过重复迁移。
#[cfg(not(feature = "verge-dev"))]
pub const LEGACY_APP_ID: &str = "io.github.clash-verge-rev.clash-verge-rev";
#[cfg(feature = "verge-dev")]
pub const LEGACY_APP_ID: &str = "io.github.clash-verge-rev.clash-verge-rev.dev";

pub static PORTABLE_FLAG: OnceCell<bool> = OnceCell::new();

pub static CLASH_CONFIG: &str = "config.yaml";
pub static VERGE_CONFIG: &str = "verge.yaml";
pub static PROFILE_YAML: &str = "profiles.yaml";

/// init portable flag — portable mode is active when a `Data/` directory
/// exists (or can be created) in the same folder as the executable.
/// On first run the directory is created automatically so the user never
/// has to do it manually.
pub fn init_portable_flag() -> Result<()> {
    use tauri::utils::platform::current_exe;

    let app_exe = current_exe()?;
    if let Some(dir) = app_exe.parent() {
        let data_dir = PathBuf::from(dir).join("Data");
        // Auto-create Data/ on first launch so portable mode self-activates.
        if !data_dir.exists() {
            let _ = fs::create_dir_all(&data_dir);
        }
        if data_dir.exists() && data_dir.is_dir() {
            PORTABLE_FLAG.get_or_init(|| true);
            return Ok(());
        }
    }
    PORTABLE_FLAG.get_or_init(|| false);
    Ok(())
}

/// Returns the portable data root: `<exe_dir>/Data/`.
/// Only valid when `PORTABLE_FLAG` is true.
pub fn portable_data_root() -> Result<PathBuf> {
    use tauri::utils::platform::current_exe;
    let app_exe = current_exe()?;
    let app_exe = dunce::canonicalize(app_exe)?;
    let exe_dir = app_exe
        .parent()
        .ok_or_else(|| anyhow::anyhow!("failed to get exe directory"))?;
    Ok(PathBuf::from(exe_dir).join("Data"))
}

/// 便携版应用数据根目录下的「配置与状态」目录：`Data/app/`。
pub fn portable_roaming_dir() -> Result<PathBuf> {
    Ok(portable_data_root()?.join("app"))
}

/// 是否能在该目录下创建用户数据（`Program Files` 下常见无写权限）。
#[cfg(target_os = "windows")]
fn webview_user_data_dir_writable(dir: &std::path::Path) -> bool {
    let _ = fs::create_dir_all(dir);
    let probe = dir.join(".clash_verge_webview_write_probe");
    match std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&probe)
    {
        Ok(_) => {
            let _ = fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

/// 便携模式：WebView2 用户数据（`EBWebView` 等）使用 `Data/webview/`；若该目录**不可写**再回退 `%LOCALAPPDATA%\ClashVergeRev\`。
#[cfg(target_os = "windows")]
pub fn effective_portable_webview_user_data_dir() -> Result<PathBuf> {
    let under_data = portable_data_root()?.join("webview");
    if webview_user_data_dir_writable(&under_data) {
        return Ok(under_data);
    }
    windows_portable_local_webview_root()
}

/// 非便携 / 便携但 `Data\webview` 不可写时：WebView2 使用 `%LOCALAPPDATA%\ClashVergeRev\`。
/// 若曾落在 `ClashVergeRev\webview\` 子目录，将内容合入本根。
#[cfg(target_os = "windows")]
pub fn windows_portable_local_webview_root() -> Result<PathBuf> {
    let local = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("LOCALAPPDATA not set"))?;
    let p = local.join("ClashVergeRev");
    let _ = fs::create_dir_all(&p);
    let old_sub = p.join("webview");
    if old_sub.is_dir() {
        let mut log: Vec<String> = Vec::new();
        migrate_old_portable_dir(&old_sub, &p, &mut log);
    }
    Ok(p)
}

/// 与 `WEBVIEW2_USER_DATA_FOLDER` 一致；**必须始终返回可写路径**。
/// 若返回失败路径或漏设 `data_directory`，Tauri 2 会在 Windows 上回退为
/// `%LOCALAPPDATA%\<identifier>\`（例如 `io.github.clash-verge-rev.clash-verge-rev`），与「仅部分解析成功」的 `Err` 表现相同。
#[cfg(target_os = "windows")]
pub fn resolve_windows_webview_user_data_folder() -> PathBuf {
    if let Ok(s) = std::env::var("WEBVIEW2_USER_DATA_FOLDER")
        && !s.is_empty()
    {
        return PathBuf::from(s);
    }
    if *PORTABLE_FLAG.get().unwrap_or(&false)
        && let Ok(p) = effective_portable_webview_user_data_dir()
    {
        return p;
    }
    if let Ok(p) = windows_portable_local_webview_root() {
        return p;
    }
    // 无 LOCALAPPDATA 的极少数环境
    let p = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("ClashVergeRev");
    let _ = fs::create_dir_all(&p);
    p
}

/// WebView2 用户数据目录。Windows 便携模式下与 `bootstrap_portable_environment` 使用相同解析逻辑。
pub fn portable_webview2_dir() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    if *PORTABLE_FLAG.get().unwrap_or(&false) {
        return effective_portable_webview_user_data_dir();
    }
    Ok(portable_data_root()?.join("webview"))
}

/// Windows 便携版：在 Tauri 初始化前设置 `WEBVIEW2_USER_DATA_FOLDER`，
/// 并将旧版布局（含 exe 旁 `.config/<APP_ID>`、`Data/Roaming/<APP_ID>` 等）迁移到 `Data/app/` 与 `Data/webview/`。
#[cfg(target_os = "windows")]
pub fn bootstrap_portable_environment() {
    let flag = PORTABLE_FLAG.get().unwrap_or(&false);
    if !*flag {
        return;
    }

    let data_root = match portable_data_root() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[portable] failed to get data root: {e}");
            return;
        }
    };

    let mut log_lines: Vec<String> = Vec::new();

    let app_dir = data_root.join("app");
    let webview2_dir = match effective_portable_webview_user_data_dir() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[portable] effective_portable_webview_user_data_dir failed: {e}");
            return;
        }
    };
    if webview2_dir != data_root.join("webview") {
        log_lines.push(format!(
            "webview2_dir fallback (e.g. Program Files not writable): {}",
            webview2_dir.display()
        ));
    }

    if let Some(exe_parent) = data_root.parent() {
        migrate_old_portable_dir(&exe_parent.join(".config").join(APP_ID), &app_dir, &mut log_lines);
    }

    migrate_old_portable_dir(&data_root.join(APP_ID), &app_dir, &mut log_lines);
    if LEGACY_APP_ID != APP_ID {
        migrate_old_portable_dir(&data_root.join(LEGACY_APP_ID), &app_dir, &mut log_lines);
    }
    migrate_old_portable_dir(
        &data_root.join("system_settings").join(APP_ID),
        &app_dir,
        &mut log_lines,
    );
    migrate_old_portable_dir(
        &data_root.join("system_settings").join(LEGACY_APP_ID),
        &app_dir,
        &mut log_lines,
    );
    migrate_old_portable_dir(&data_root.join("app_local").join(APP_ID), &webview2_dir, &mut log_lines);
    migrate_old_portable_dir(
        &data_root.join("app_local").join(LEGACY_APP_ID),
        &webview2_dir,
        &mut log_lines,
    );
    migrate_old_portable_dir(&data_root.join("Roaming").join(APP_ID), &app_dir, &mut log_lines);
    migrate_old_portable_dir(&data_root.join("Roaming").join(LEGACY_APP_ID), &app_dir, &mut log_lines);
    migrate_old_portable_dir(&data_root.join("Local").join(APP_ID), &webview2_dir, &mut log_lines);
    migrate_old_portable_dir(
        &data_root.join("Local").join(LEGACY_APP_ID),
        &webview2_dir,
        &mut log_lines,
    );

    match fs::create_dir_all(&webview2_dir) {
        Ok(()) => log_lines.push(format!("create_dir_all OK: {}", webview2_dir.display())),
        Err(e) => log_lines.push(format!("create_dir_all FAIL: {} — {e}", webview2_dir.display())),
    }
    match fs::create_dir_all(&app_dir) {
        Ok(()) => log_lines.push(format!("create_dir_all OK: {}", app_dir.display())),
        Err(e) => log_lines.push(format!("create_dir_all FAIL: {} — {e}", app_dir.display())),
    }

    unsafe {
        std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", &webview2_dir);
    }
    eprintln!("[portable] WEBVIEW2_USER_DATA_FOLDER = {}", webview2_dir.display());
}

/// 安装版 / 非便携：在 Tauri 初始化前设置 `WEBVIEW2_USER_DATA_FOLDER` 为 `%LOCALAPPDATA%\ClashVergeRev\`，
/// 避免 WebView2 使用默认的 `%LOCALAPPDATA%\<identifier>\`。
#[cfg(target_os = "windows")]
pub fn bootstrap_windows_non_portable_webview() {
    if *PORTABLE_FLAG.get().unwrap_or(&false) {
        return;
    }
    if let Ok(p) = windows_portable_local_webview_root() {
        let _ = fs::create_dir_all(&p);
        unsafe {
            std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", &p);
        }
    }
}

/// 在**进程**内最早调用：设置便携标志、写入 `WEBVIEW2_USER_DATA_FOLDER`（与 `resolve_windows_webview_user_data_folder` 一致）。
/// 在 `main` 与 `lib::run` 两处分发，避免将来若有其它入口未跑 `main` 时仍落回 `Local\<identifier>\`。
#[cfg(target_os = "windows")]
pub fn apply_windows_webview2_user_data_bootstrap() {
    let _ = init_portable_flag();
    if *PORTABLE_FLAG.get().unwrap_or(&false) {
        bootstrap_portable_environment();
    } else {
        bootstrap_windows_non_portable_webview();
    }
}

/// Moves the contents of `src` into `dst` if `src` is a plain directory that
/// exists.  Skips files already present in `dst`.  Does nothing if `src` is a
/// junction or does not exist.  Called during version upgrades where the
/// portable directory layout changed.
#[cfg(target_os = "windows")]
fn migrate_old_portable_dir(src: &PathBuf, dst: &PathBuf, log: &mut Vec<String>) {
    if junction::exists(src).unwrap_or(false) {
        log.push(format!("migrate skip (junction): {}", src.display()));
        return;
    }
    if !src.is_dir() {
        return; // not interesting enough to log
    }
    let _ = fs::create_dir_all(dst);
    let mut count = 0u32;
    if let Ok(entries) = fs::read_dir(src) {
        for entry in entries.flatten() {
            let target_path = dst.join(entry.file_name());
            if !target_path.exists() {
                let _ = fs::rename(entry.path(), &target_path);
                count += 1;
            }
        }
    }
    log.push(format!(
        "migrate {} -> {}: {count} files moved",
        src.display(),
        dst.display()
    ));
}

/// 仅删除旧版便携模式在 `%APPDATA%` / `%LOCALAPPDATA%` 下创建的 **NTFS 目录联接**。
/// 不要对同路径的「普通目录」整树 `remove_dir_all`：若该目录已是 WebView2 用户数据，退出时会误删（含 `.cookies`）。
#[cfg(target_os = "windows")]
pub fn cleanup_junctions() {
    let flag = PORTABLE_FLAG.get().unwrap_or(&false);
    if !*flag {
        return;
    }

    let real_local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    let real_roaming = std::env::var_os("APPDATA").map(PathBuf::from);

    for base in [real_local, real_roaming].into_iter().flatten() {
        for id in [APP_ID, LEGACY_APP_ID] {
            let link = base.join(id);
            for _ in 0..2 {
                if junction::exists(&link).unwrap_or(false) {
                    let _ = junction::delete(&link);
                } else {
                    break;
                }
            }
        }
    }
}

/// get the verge app home dir
pub fn app_home_dir() -> Result<PathBuf> {
    let flag = PORTABLE_FLAG.get().unwrap_or(&false);
    if *flag {
        // 便携版：配置与状态在 exe 同目录 `Data/app/` 下（与 iGame-for-Windows 一致）
        return portable_roaming_dir();
    }

    // 避免在Handle未初始化时崩溃
    let app_handle = handle::Handle::app_handle();

    match app_handle.path().data_dir() {
        Ok(dir) => Ok(dir.join(APP_ID)),
        Err(e) => {
            logging!(error, Type::File, "Failed to get the app home directory: {e}");
            Err(anyhow::anyhow!("Failed to get the app homedirectory"))
        }
    }
}

/// get the resources dir
pub fn app_resources_dir() -> Result<PathBuf> {
    // 避免在Handle未初始化时崩溃
    let app_handle = handle::Handle::app_handle();

    match app_handle.path().resource_dir() {
        Ok(dir) => Ok(dir.join("resources")),
        Err(e) => {
            logging!(error, Type::File, "Failed to get the resource directory: {e}");
            Err(anyhow::anyhow!("Failed to get the resource directory"))
        }
    }
}

/// profiles dir
pub fn app_profiles_dir() -> Result<PathBuf> {
    Ok(app_home_dir()?.join("profiles"))
}

/// icons dir
pub fn app_icons_dir() -> Result<PathBuf> {
    Ok(app_home_dir()?.join("icons"))
}

pub fn find_target_icons(target: &str) -> Result<Option<String>> {
    let icons_dir = app_icons_dir()?;
    let icon_path = fs::read_dir(&icons_dir)?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .find(|path| {
            let prefix_matches = path
                .file_prefix()
                .and_then(|p| p.to_str())
                .is_some_and(|prefix| prefix.starts_with(target));
            let ext_matches = path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("ico") || ext.eq_ignore_ascii_case("png"));
            prefix_matches && ext_matches
        });

    icon_path.map(|path| path_to_str(&path).map(|s| s.into())).transpose()
}

/// logs dir
pub fn app_logs_dir() -> Result<PathBuf> {
    Ok(app_home_dir()?.join("logs"))
}

// latest verge log
pub fn app_latest_log() -> Result<PathBuf> {
    Ok(app_logs_dir()?.join("latest.log"))
}

/// local backups dir
pub fn local_backup_dir() -> Result<PathBuf> {
    let dir = app_home_dir()?.join(BACKUP_DIR);
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn clash_path() -> Result<PathBuf> {
    Ok(app_home_dir()?.join(CLASH_CONFIG))
}

pub fn verge_path() -> Result<PathBuf> {
    Ok(app_home_dir()?.join(VERGE_CONFIG))
}

pub fn profiles_path() -> Result<PathBuf> {
    Ok(app_home_dir()?.join(PROFILE_YAML))
}

#[cfg(target_os = "macos")]
pub fn service_path() -> Result<PathBuf> {
    let res_dir = app_resources_dir()?;
    Ok(res_dir.join("clash-verge-service"))
}

#[cfg(windows)]
pub fn service_path() -> Result<PathBuf> {
    let res_dir = app_resources_dir()?;
    Ok(res_dir.join("clash-verge-service.exe"))
}

pub fn sidecar_log_dir() -> Result<PathBuf> {
    let log_dir = app_logs_dir()?.join("sidecar");
    let _ = std::fs::create_dir_all(&log_dir);

    Ok(log_dir)
}

pub fn service_log_dir() -> Result<PathBuf> {
    let log_dir = app_logs_dir()?.join("service");
    let _ = std::fs::create_dir_all(&log_dir);

    Ok(log_dir)
}

pub fn clash_latest_log() -> Result<PathBuf> {
    Ok(sidecar_log_dir()?.join("sidecar_latest.log"))
}

pub fn path_to_str(path: &PathBuf) -> Result<&str> {
    let path_str = path
        .as_os_str()
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("failed to get path from {:?}", path))?;
    Ok(path_str)
}

pub fn get_encryption_key() -> Result<Vec<u8>> {
    let app_dir = app_home_dir()?;
    let key_path = app_dir.join(".encryption_key");

    if key_path.exists() {
        // Read existing key
        fs::read(&key_path).map_err(|e| anyhow::anyhow!("Failed to read encryption key: {}", e))
    } else {
        // Generate and save new key
        let mut key = vec![0u8; 32];
        getrandom::fill(&mut key)?;

        // Ensure directory exists
        if let Some(parent) = key_path.parent() {
            fs::create_dir_all(parent).map_err(|e| anyhow::anyhow!("Failed to create key directory: {}", e))?;
        }
        // Save key
        fs::write(&key_path, &key).map_err(|e| anyhow::anyhow!("Failed to save encryption key: {}", e))?;
        Ok(key)
    }
}

#[cfg(unix)]
pub fn ensure_mihomo_safe_dir() -> Option<PathBuf> {
    iter::once("/tmp")
        .map(PathBuf::from)
        .find(|path| path.exists())
        .or_else(|| {
            std::env::var_os("HOME").and_then(|home| {
                let home_config = PathBuf::from(home).join(".config");
                if home_config.exists() || fs::create_dir_all(&home_config).is_ok() {
                    Some(home_config)
                } else {
                    logging!(error, Type::File, "Failed to create safe directory: {home_config:?}");
                    None
                }
            })
        })
}

#[cfg(unix)]
pub fn ipc_path() -> Result<PathBuf> {
    ensure_mihomo_safe_dir()
        .map(|base_dir| base_dir.join("verge").join("verge-mihomo.sock"))
        .or_else(|| {
            app_home_dir()
                .ok()
                .map(|dir| dir.join("verge").join("verge-mihomo.sock"))
        })
        .ok_or_else(|| anyhow::anyhow!("Failed to determine ipc path"))
}

#[cfg(target_os = "windows")]
pub fn ipc_path() -> Result<PathBuf> {
    Ok(PathBuf::from(r"\\.\pipe\verge-mihomo"))
}
#[async_trait]
pub trait PathBufExec {
    async fn remove_if_exists(&self) -> Result<()>;
}

#[async_trait]
impl PathBufExec for PathBuf {
    async fn remove_if_exists(&self) -> Result<()> {
        if self.exists() {
            tokio::fs::remove_file(self).await?;
            logging!(info, Type::File, "Removed file: {:?}", self);
        }
        Ok(())
    }
}
