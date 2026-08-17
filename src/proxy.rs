/// Proxy detection and resolution for git commands.
///
/// Strategy:
/// 1. If --proxy off  → never use a proxy
/// 2. If --proxy <url> → always use that URL on network failure
/// 3. If --proxy auto (default) → only probe when a git command fails with a
///    network error; probe order:
///      env vars (HTTPS_PROXY → HTTP_PROXY → ALL_PROXY) → OS system proxy

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

/// A detected proxy URL together with its protocol type.
/// The scheme drives which env vars are injected into the git subprocess.
#[derive(Debug, Clone)]
pub struct ProxyUrl {
    /// Full proxy URL, e.g. "http://127.0.0.1:7890" or "socks5://127.0.0.1:1080"
    pub url: String,
    /// Whether this is a SOCKS proxy (git reads ALL_PROXY for SOCKS)
    pub is_socks: bool,
}

impl ProxyUrl {
    #[cfg(not(windows))]
    fn http(url: String) -> Self {
        ProxyUrl {
            url,
            is_socks: false,
        }
    }

    #[cfg(not(windows))]
    fn socks(url: String) -> Self {
        ProxyUrl {
            url,
            is_socks: true,
        }
    }

    /// Infer proxy type from URL scheme
    pub fn from_url(url: String) -> Self {
        let is_socks = url.starts_with("socks");
        ProxyUrl { url, is_socks }
    }
}

/// Probe all available proxy sources and return the first found proxy.
/// Order: env vars (HTTPS_PROXY → HTTP_PROXY → ALL_PROXY) → OS system proxy
pub fn detect_proxy() -> Option<ProxyUrl> {
    // 1. Environment variables (check both upper and lower case)
    let env_names = [
        "HTTPS_PROXY",
        "https_proxy",
        "HTTP_PROXY",
        "http_proxy",
        "ALL_PROXY",
        "all_proxy",
    ];
    for name in &env_names {
        if let Ok(val) = std::env::var(name) {
            let val = val.trim().to_string();
            if !val.is_empty() {
                return Some(ProxyUrl::from_url(val));
            }
        }
    }

    // 2. OS-level system proxy
    detect_system_proxy()
}

// ── Windows ─────────────────────────────────────────────────────────────────

/// Read the OS system proxy setting (Windows registry).
#[cfg(windows)]
fn detect_system_proxy() -> Option<ProxyUrl> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};

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
    let raw = extract_proxy_for_protocol(&proxy_server, "https")
        .or_else(|| extract_proxy_for_protocol(&proxy_server, "http"))
        .unwrap_or_else(|| proxy_server.clone());

    // Ensure the URL has a scheme
    let url = if raw.contains("://") {
        raw
    } else {
        format!("http://{}", raw)
    };

    Some(ProxyUrl::from_url(url))
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

// ── macOS / Linux ────────────────────────────────────────────────────────────

#[cfg(not(windows))]
fn detect_system_proxy() -> Option<ProxyUrl> {
    // macOS: networksetup covers HTTPS, SOCKS and HTTP system proxy settings.
    // Linux: gsettings covers GNOME SOCKS and HTTP proxy settings.
    // In both cases env vars (checked earlier) cover the majority of real users.
    detect_macos_proxy().or_else(detect_linux_proxy)
}

// ── macOS ────────────────────────────────────────────────────────────────────

/// Query a single `networksetup` proxy type for one network interface.
/// Returns the proxy URL string if the proxy is enabled, otherwise None.
#[cfg(not(windows))]
fn query_networksetup(iface: &str, flag: &str, scheme: &str) -> Option<String> {
    let out = std::process::Command::new("networksetup")
        .args([flag, iface])
        .output()
        .ok()?;

    if !out.status.success() {
        return None;
    }

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
        Some(format!("{}://{}:{}", scheme, server, port))
    } else {
        None
    }
}

#[cfg(not(windows))]
fn detect_macos_proxy() -> Option<ProxyUrl> {
    let interfaces = ["Wi-Fi", "Ethernet", "en0", "en1"];

    // Priority order:
    //   HTTPS  — most relevant for git remote operations
    //   SOCKS  — covers all protocols, preferred over plain HTTP
    //   HTTP   — plain HTTP proxy as fallback
    let proxy_types: &[(&str, &str)] = &[
        ("-getsecurewebproxy", "https"),
        ("-getsocksfirewallproxy", "socks5"),
        ("-getwebproxy", "http"),
    ];

    for iface in &interfaces {
        for (flag, scheme) in proxy_types {
            if let Some(url) = query_networksetup(iface, flag, scheme) {
                return Some(if *scheme == "socks5" {
                    ProxyUrl::socks(url)
                } else {
                    ProxyUrl::http(url)
                });
            }
        }
    }
    None
}

// ── Linux (GNOME gsettings) ──────────────────────────────────────────────────

#[cfg(not(windows))]
fn detect_linux_proxy() -> Option<ProxyUrl> {
    // Only GNOME gsettings is supported. KDE and other DEs are not covered;
    // those users typically rely on env vars which are caught earlier.
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

    // Try SOCKS first (covers all protocols), then fall back to HTTP
    if let Some(proxy) = detect_linux_socks_proxy() {
        return Some(proxy);
    }
    detect_linux_http_proxy()
}

#[cfg(not(windows))]
fn gsettings_get(schema: &str, key: &str) -> Option<String> {
    let out = std::process::Command::new("gsettings")
        .args(["get", schema, key])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let val = String::from_utf8_lossy(&out.stdout)
        .trim()
        .trim_matches('\'')
        .to_string();
    if val.is_empty() { None } else { Some(val) }
}

#[cfg(not(windows))]
fn detect_linux_socks_proxy() -> Option<ProxyUrl> {
    let host = gsettings_get("org.gnome.system.proxy.socks", "host")?;
    let port = gsettings_get("org.gnome.system.proxy.socks", "port")?;
    if port == "0" {
        return None;
    }
    Some(ProxyUrl::socks(format!("socks5://{}:{}", host, port)))
}

#[cfg(not(windows))]
fn detect_linux_http_proxy() -> Option<ProxyUrl> {
    let host = gsettings_get("org.gnome.system.proxy.http", "host")?;
    let port = gsettings_get("org.gnome.system.proxy.http", "port")?;
    if port == "0" {
        return None;
    }
    Some(ProxyUrl::http(format!("http://{}:{}", host, port)))
}

// ── Network error detection ───────────────────────────────────────────────────

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
        "fatal: repository", // often network-related for remote repos
        "error: server certificate",
    ];
    network_patterns.iter().any(|p| lower.contains(p))
}
