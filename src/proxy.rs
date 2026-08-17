/// Proxy detection and resolution for git commands.
///
/// Strategy:
/// 1. If --proxy off  → never use a proxy
/// 2. If --proxy <url> → always use that URL
/// 3. If --proxy auto (default) → only probe when a git command fails with a network error;
///    probe order: env vars (HTTPS_PROXY → HTTP_PROXY → ALL_PROXY) → OS system proxy

/// Represents the user's --proxy choice
#[derive(Debug, Clone)]
pub enum ProxyMode {
    /// Default: auto-detect only on network failure
    Auto,
    /// Explicit proxy URL supplied by user
    Manual(String),
    /// Proxy disabled
    Off,
}

impl ProxyMode {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "auto" => ProxyMode::Auto,
            "off" | "none" | "no" => ProxyMode::Off,
            url => ProxyMode::Manual(url.to_string()),
        }
    }
}

/// Probe all available proxy sources and return the first found URL.
/// Order: HTTPS_PROXY → HTTP_PROXY → ALL_PROXY → OS system proxy
pub fn detect_proxy() -> Option<String> {
    // 1. Environment variables (case-insensitive; check both upper and lower)
    let env_names = [
        "HTTPS_PROXY", "https_proxy",
        "HTTP_PROXY",  "http_proxy",
        "ALL_PROXY",   "all_proxy",
    ];
    for name in &env_names {
        if let Ok(val) = std::env::var(name) {
            let val = val.trim().to_string();
            if !val.is_empty() {
                return Some(val);
            }
        }
    }

    // 2. OS-level system proxy
    detect_system_proxy()
}

/// Read the OS system proxy setting.
#[cfg(windows)]
fn detect_system_proxy() -> Option<String> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey_with_flags(
            r"Software\Microsoft\Windows\CurrentVersion\Internet Settings",
            KEY_READ,
        )
        .ok()?;

    // ProxyEnable = 1 means proxy is active
    let enabled: u32 = key.get_value("ProxyEnable").ok()?;
    if enabled == 0 {
        return None;
    }

    let proxy_server: String = key.get_value("ProxyServer").ok()?;
    let proxy_server = proxy_server.trim().to_string();
    if proxy_server.is_empty() {
        return None;
    }

    // ProxyServer can be:
    //   "host:port"                     → apply to all protocols
    //   "http=host:port;https=host:port" → per-protocol
    // Extract https first, then http, then fallback to raw value
    let url = extract_proxy_for_protocol(&proxy_server, "https")
        .or_else(|| extract_proxy_for_protocol(&proxy_server, "http"))
        .unwrap_or_else(|| proxy_server.clone());

    // Ensure the URL has a scheme
    let url = if url.contains("://") {
        url
    } else {
        format!("http://{}", url)
    };

    Some(url)
}

#[cfg(windows)]
fn extract_proxy_for_protocol(proxy_server: &str, protocol: &str) -> Option<String> {
    // Format: "http=1.2.3.4:8080;https=1.2.3.4:8080"
    for part in proxy_server.split(';') {
        let part = part.trim();
        if let Some(rest) = part.strip_prefix(&format!("{}=", protocol)) {
            let rest = rest.trim().to_string();
            if !rest.is_empty() {
                return Some(if rest.contains("://") {
                    rest
                } else {
                    format!("http://{}", rest)
                });
            }
        }
    }
    None
}

#[cfg(not(windows))]
fn detect_system_proxy() -> Option<String> {
    // macOS: try `networksetup -getwebproxy` via subprocess
    // Linux: try gsettings
    // Both are best-effort; env vars above already cover most cases.
    detect_macos_proxy().or_else(detect_linux_proxy)
}

#[cfg(not(windows))]
fn detect_macos_proxy() -> Option<String> {
    // Try `networksetup -getwebproxy <interface>` for common interfaces
    let interfaces = ["Wi-Fi", "Ethernet", "en0", "en1"];
    for iface in &interfaces {
        if let Ok(out) = std::process::Command::new("networksetup")
            .args(["-getwebproxy", iface])
            .output()
        {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                let mut enabled = false;
                let mut server = String::new();
                let mut port = String::new();
                for line in text.lines() {
                    if line.starts_with("Enabled:") {
                        enabled = line.contains("Yes");
                    } else if line.starts_with("Server:") {
                        server = line["Server:".len()..].trim().to_string();
                    } else if line.starts_with("Port:") {
                        port = line["Port:".len()..].trim().to_string();
                    }
                }
                if enabled && !server.is_empty() && port != "0" {
                    return Some(format!("http://{}:{}", server, port));
                }
            }
        }
    }
    None
}

#[cfg(not(windows))]
fn detect_linux_proxy() -> Option<String> {
    // Try gsettings (GNOME)
    let output = std::process::Command::new("gsettings")
        .args(["get", "org.gnome.system.proxy", "mode"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let mode = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if mode != "'manual'" {
        return None;
    }

    let host_out = std::process::Command::new("gsettings")
        .args(["get", "org.gnome.system.proxy.http", "host"])
        .output()
        .ok()?;
    let port_out = std::process::Command::new("gsettings")
        .args(["get", "org.gnome.system.proxy.http", "port"])
        .output()
        .ok()?;

    let host = String::from_utf8_lossy(&host_out.stdout)
        .trim()
        .trim_matches('\'')
        .to_string();
    let port = String::from_utf8_lossy(&port_out.stdout)
        .trim()
        .to_string();

    if host.is_empty() || port == "0" {
        return None;
    }

    Some(format!("http://{}:{}", host, port))
}

/// Determine whether a git command's stderr indicates a network-level failure.
/// Returns true for connection refused, timeout, SSL errors, resolve failures, etc.
pub fn is_network_error(stderr: &str) -> bool {
    let lower = stderr.to_lowercase();
    let network_patterns = [
        "could not resolve host",
        "could not connect",
        "connection refused",
        "connection timed out",
        "operation timed out",
        "network is unreachable",
        "failed to connect",
        "unable to access",
        "ssl certificate",
        "ssl_connect",
        "openssl ssl_connect",
        "curl error",
        "recv failure",
        "send failure",
        "gnutls",
        "fatal: repository",          // often network-related for remote repos
        "error: server certificate",
    ];
    network_patterns.iter().any(|p| lower.contains(p))
}
