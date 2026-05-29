use super::{CoreManager, RunningMode};
use crate::cmd::StringifyErr as _;
use crate::config::{Config, IVerge};
use crate::core::handle::Handle;
use crate::core::manager::CLASH_LOGGER;
use anyhow::Result;
use clash_verge_logging::{Type, logging};
use scopeguard::defer;
use smartstring::alias::String;
use tauri_plugin_clash_verge_sysinfo;

impl CoreManager {
    pub async fn start_core(&self) -> Result<()> {
        if matches!(*self.get_running_mode(), RunningMode::Sidecar) {
            logging!(info, Type::Core, "core 已在运行，跳过重复启动");
            return Ok(());
        }
        self.prepare_startup();
        defer! {
            self.after_core_process();
        }

        self.start_core_by_sidecar().await
    }

    pub async fn stop_core(&self) -> Result<()> {
        defer! {
            self.after_core_process();
        }

        match *self.get_running_mode() {
            RunningMode::Sidecar => {
                self.stop_core_by_sidecar();
            }
            RunningMode::NotRunning => {}
        }
        CLASH_LOGGER.clear_logs().await;
        Ok(())
    }

    pub async fn restart_core(&self) -> Result<()> {
        logging!(info, Type::Core, "Restarting core");
        self.stop_core().await?;
        self.start_core().await
    }

    pub async fn change_core(&self, clash_core: &String) -> Result<(), String> {
        if !IVerge::VALID_CLASH_CORES.contains(&clash_core.as_str()) {
            return Err(format!("Invalid clash core: {}", clash_core).into());
        }

        Config::verge().await.edit_draft(|d| {
            d.clash_core = Some(clash_core.to_owned());
        });
        Config::verge().await.apply();

        let verge_data = Config::verge().await.latest_arc();
        verge_data.save_file().await.map_err(|e| e.to_string())?;

        self.update_config_checked().await.stringify_err()?;
        Ok(())
    }

    fn prepare_startup(&self) {
        self.set_running_mode(RunningMode::Sidecar);
    }

    fn after_core_process(&self) {
        let app_handle = Handle::app_handle();
        tauri_plugin_clash_verge_sysinfo::set_app_core_mode(app_handle, self.get_running_mode().to_string());
    }
}
