#[derive(Debug, Clone, Copy)]
pub enum HostPlatform {
    MacOs,
    Linux,
    Windows,
    Unknown,
}

pub fn detect_platform() -> HostPlatform {
    if cfg!(target_os = "macos") {
        HostPlatform::MacOs
    } else if cfg!(target_os = "linux") {
        HostPlatform::Linux
    } else if cfg!(target_os = "windows") {
        HostPlatform::Windows
    } else {
        HostPlatform::Unknown
    }
}

pub fn default_term_type() -> &'static str {
    match detect_platform() {
        HostPlatform::MacOs | HostPlatform::Linux => "xterm-256color",
        HostPlatform::Windows => "xterm",
        HostPlatform::Unknown => "xterm",
    }
}
