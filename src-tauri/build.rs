fn main() {
    #[cfg(feature = "clippy")]
    {
        println!("cargo:warning=Skipping tauri_build during Clippy");
    }

    #[cfg(not(feature = "clippy"))]
    {
        println!("cargo:rerun-if-changed=icons/icon.ico");

        #[cfg(target_os = "windows")]
        {
            println!("cargo:rerun-if-changed=icons/128x128@2x.png");
            println!("cargo:rerun-if-changed=windows-app-manifest.xml");
            kill_sidecar_processes_for_tauri_build();
        }

        // Windows：嵌入 requireAdministrator，双击启动即走 UAC 提权（TUN 等无需再单独处理进程令牌）。
        let win_attrs = tauri_build::WindowsAttributes::new_without_app_manifest()
            .app_manifest(include_str!("windows-app-manifest.xml"));
        let attrs = tauri_build::Attributes::new().windows_attributes(win_attrs);

        if let Err(error) = tauri_build::try_build(attrs) {
            let error = format!("{error:#}");
            println!("{error}");
            if error.starts_with("unknown field") {
                print!(
                    "found an unknown configuration field. This usually means that you are using a CLI version that is newer than `tauri-build` and is incompatible. "
                );
                println!("Please try updating the Rust crates by running `cargo update` in the Tauri app folder.");
            }
            std::process::exit(1);
        }
    }
}

/// 编译时 tauri-build 会处理 `sidecar\verge-mihomo-*.exe`；若该文件正被运行中的同名进程占用，会报「拒绝访问」。
/// 与运行时 `main` 里的清理一致：在调用 `tauri_build::try_build` 之前先结束进程（含直接 `cargo build` 未走 npm prebuild 的情况）。
#[cfg(all(target_os = "windows", not(feature = "clippy")))]
fn kill_sidecar_processes_for_tauri_build() {
    use std::os::windows::process::CommandExt as _;
    use std::path::PathBuf;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
    let taskkill = PathBuf::from(root).join("System32").join("taskkill.exe");
    if !taskkill.is_file() {
        return;
    }

    for name in ["verge-mihomo.exe", "verge-mihomo-alpha.exe"] {
        let _ = std::process::Command::new(&taskkill)
            .args(["/F", "/T", "/IM", name])
            .creation_flags(CREATE_NO_WINDOW)
            .status();
    }
    std::thread::sleep(std::time::Duration::from_millis(400));
}
