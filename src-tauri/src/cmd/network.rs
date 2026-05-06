use super::CmdResult;
use crate::cmd::StringifyErr as _;
use crate::config::Config;
use crate::config::IClashTemp;
use crate::core::sysopt::Sysopt;
use clash_verge_logging::{Type, logging};
use gethostname::gethostname;
use network_interface::NetworkInterface;
use serde::Serialize;
use serde_yaml_ng::Mapping;
use std::collections::HashMap;
use std::error::Error as _;
use std::net::TcpListener;
use std::time::Duration;
use sysproxy::{Autoproxy, Sysproxy};
use tauri_plugin_clash_verge_sysinfo;

/// get the system proxy
#[tauri::command]
pub async fn get_sys_proxy() -> CmdResult<Mapping> {
    logging!(debug, Type::Network, "异步获取系统代理配置");

    Sysopt::global().wait_idle().await;
    let sys_proxy = Sysproxy::get_system_proxy().stringify_err()?;
    let Sysproxy {
        ref host,
        ref bypass,
        ref port,
        ref enable,
    } = sys_proxy;

    let mut map = Mapping::new();
    map.insert("enable".into(), (*enable).into());
    map.insert("server".into(), format!("{}:{}", host, port).into());
    map.insert("bypass".into(), bypass.as_str().into());

    logging!(
        debug,
        Type::Network,
        "返回系统代理配置: enable={}, {}:{}",
        sys_proxy.enable,
        sys_proxy.host,
        sys_proxy.port
    );
    Ok(map)
}

/// 获取自动代理配置
#[tauri::command]
pub async fn get_auto_proxy() -> CmdResult<Mapping> {
    Sysopt::global().wait_idle().await;
    let auto_proxy = Autoproxy::get_auto_proxy().stringify_err()?;
    let Autoproxy { ref enable, ref url } = auto_proxy;

    let mut map = Mapping::new();
    map.insert("enable".into(), (*enable).into());
    map.insert("url".into(), url.as_str().into());

    logging!(
        debug,
        Type::Network,
        "返回自动代理配置（缓存）: enable={}, url={}",
        auto_proxy.enable,
        auto_proxy.url
    );
    Ok(map)
}

/// 获取系统主机名
#[tauri::command]
pub fn get_system_hostname() -> String {
    // 获取系统主机名，处理可能的非UTF-8字符
    match gethostname().into_string() {
        Ok(name) => name,
        Err(os_string) => {
            // 对于包含非UTF-8的主机名，使用调试格式化
            let fallback = format!("{os_string:?}");
            // 去掉可能存在的引号
            fallback.trim_matches('"').to_string()
        }
    }
}

/// 获取网络接口列表
#[tauri::command]
pub fn get_network_interfaces() -> Vec<String> {
    tauri_plugin_clash_verge_sysinfo::list_network_interfaces()
}

/// 获取网络接口详细信息
#[tauri::command]
pub fn get_network_interfaces_info() -> CmdResult<Vec<NetworkInterface>> {
    use network_interface::{NetworkInterface, NetworkInterfaceConfig as _};

    let names = get_network_interfaces();
    let interfaces = NetworkInterface::show().stringify_err()?;

    let mut result = Vec::new();

    for interface in interfaces {
        if names.contains(&interface.name) {
            result.push(interface);
        }
    }

    Ok(result)
}

#[tauri::command]
pub fn is_port_in_use(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_err()
}

fn clash_mode_lower(clash: &IClashTemp) -> String {
    clash
        .0
        .get("mode")
        .and_then(|v| v.as_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_else(|| "rule".to_string())
}

#[derive(serde::Serialize)]
pub struct IpDetectionHttpResponse {
    pub status: u16,
    pub body: String,
}

async fn ip_detection_http_request(
    url: &str,
    user_agent: &str,
    connect: std::time::Duration,
    request: std::time::Duration,
    mixed_port: Option<u16>,
) -> Result<IpDetectionHttpResponse, String> {
    use reqwest::header::ACCEPT;

    let mut builder = reqwest::Client::builder()
        .use_rustls_tls()
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(request)
        .connect_timeout(connect);

    if let Some(port) = mixed_port
        && port > 0
    {
        let proxy_url = format!("http://127.0.0.1:{port}");
        let proxy = reqwest::Proxy::all(&proxy_url).map_err(|e| e.to_string())?;
        builder = builder.proxy(proxy);
    }

    let client = builder.build().map_err(|e| e.to_string())?;
    let resp = client
        .get(url)
        .header(ACCEPT, "application/json")
        .header("User-Agent", user_agent)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    let body = resp.text().await.map_err(|e| e.to_string())?;
    Ok(IpDetectionHttpResponse { status, body })
}

/// 仅重载 OS 级系统代理，不把 `enable_system_proxy` 在配置里关断再打开（供前端在混合端口变化等场景下调用）。
#[tauri::command]
pub async fn refresh_system_proxy() -> CmdResult {
    Sysopt::global().update_sysproxy().await.map_err(|e| e.to_string())?;
    Sysopt::global().refresh_guard().await;
    Ok(())
}

#[derive(Serialize)]
pub struct PanelHttpResponse {
    pub status: u16,
    pub body: String,
}

fn reqwest_error_chain(e: &reqwest::Error) -> String {
    let mut s = e.to_string();
    let mut cur = e.source();
    while let Some(c) = cur {
        s.push_str("\n  caused by: ");
        s.push_str(&c.to_string());
        cur = c.source();
    }
    s
}

/// 首页 IP 检测等：`reqwest` + rustls 直连，与 iGame+ `panel_http_request` 一致（不经 mixed，避免核心未就绪时全失败）。
#[tauri::command]
pub async fn panel_http_request(
    method: String,
    url: String,
    body: Option<String>,
    headers: Option<HashMap<String, String>>,
    connect_timeout_ms: Option<u64>,
    request_timeout_ms: Option<u64>,
) -> CmdResult<PanelHttpResponse> {
    use reqwest::header::{ACCEPT, CONTENT_TYPE};

    let c_ms = connect_timeout_ms.unwrap_or(30_000).clamp(1_000, 120_000);
    let r_ms = request_timeout_ms.unwrap_or(45_000).max(c_ms).min(300_000);
    let connect = Duration::from_millis(c_ms);
    let request = Duration::from_millis(r_ms);
    let client = reqwest::Client::builder()
        .use_rustls_tls()
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(request)
        .connect_timeout(connect)
        .build()
        .map_err(|e| e.to_string())?;

    let method = method.to_uppercase();
    let mut req = match method.as_str() {
        "GET" => client.get(&url),
        "POST" => client.post(&url),
        "PUT" => client.put(&url),
        "PATCH" => client.patch(&url),
        "DELETE" => client.delete(&url),
        _ => return Err(format!("unsupported HTTP method: {method}").into()),
    };

    req = req.header(ACCEPT, "application/json");
    if method != "GET" && body.is_some() {
        req = req.header(CONTENT_TYPE, "application/json");
    }
    if let Some(h) = headers {
        for (k, v) in h {
            req = req.header(&k, v);
        }
    }
    if let Some(b) = body {
        req = req.body(b);
    }

    let resp = req
        .send()
        .await
        .map_err(|e| format!("reqwest: {}", reqwest_error_chain(&e)))?;
    let status = resp.status().as_u16();
    let text = resp.text().await.map_err(|e| e.to_string())?;
    Ok(PanelHttpResponse { status, body: text })
}

/// IP 检测可选路径：经 Mihomo mixed；`api.ts` 默认走 `panel_http_request` 直连。
#[tauri::command]
pub async fn ip_detection_http_get(
    url: String,
    user_agent: String,
    connect_timeout_ms: Option<u64>,
    request_timeout_ms: Option<u64>,
    use_mixed_port_proxy: Option<bool>,
) -> CmdResult<IpDetectionHttpResponse> {
    use std::time::Duration;

    let c_ms = connect_timeout_ms.unwrap_or(8_000).clamp(1_000, 120_000);
    let r_ms = request_timeout_ms.unwrap_or(10_000).max(c_ms).min(300_000);
    let connect = Duration::from_millis(c_ms);
    let request = Duration::from_millis(r_ms);

    let clash = Config::clash().await.latest_arc();
    let mode = clash_mode_lower(&clash);
    let want_mixed = match use_mixed_port_proxy {
        Some(false) => false,
        _ => mode != "direct",
    };

    let mut mixed_port: Option<u16> = None;
    if want_mixed {
        let verge = Config::verge().await.latest_arc();
        let p = match verge.verge_mixed_port {
            Some(p) if p > 0 => p,
            _ => clash.get_mixed_port(),
        };
        if p > 0 {
            mixed_port = Some(p);
        }
    }

    if let Some(port) = mixed_port {
        match ip_detection_http_request(&url, &user_agent, connect, request, Some(port)).await {
            Ok(r) => return Ok(r),
            Err(e) => {
                logging!(
                    warn,
                    Type::Network,
                    "ip_detection via mixed 127.0.0.1:{} failed: {}, retrying direct",
                    port,
                    e
                );
            }
        }
    }

    ip_detection_http_request(&url, &user_agent, connect, request, None)
        .await
        .stringify_err()
}
