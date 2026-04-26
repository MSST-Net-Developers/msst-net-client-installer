use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Os {
    Linux,
    Windows,
    MacOs,
}

impl Os {
    pub fn display_name(self) -> &'static str {
        match self {
            Os::Linux => "Linux",
            Os::Windows => "Windows",
            Os::MacOs => "macOS",
        }
    }

    pub fn platform_str(self) -> &'static str {
        match self {
            Os::Linux => "linux",
            Os::Windows => "windows",
            Os::MacOs => "macos",
        }
    }

    pub fn exe_suffix(self) -> &'static str {
        match self {
            Os::Windows => ".exe",
            _ => "",
        }
    }

    pub fn install_dir(self) -> PathBuf {
        match self {
            Os::Linux => PathBuf::from("/opt/msst-net"),
            Os::Windows => {
                let base = std::env::var("PROGRAMFILES")
                    .unwrap_or_else(|_| r"C:\Program Files".to_string());
                PathBuf::from(base).join("MSST-Net")
            }
            Os::MacOs => PathBuf::from("/usr/local/msst-net"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Arch {
    X86_64,
    Arm64,
}

impl Arch {
    pub fn arch_str(self) -> &'static str {
        match self {
            Arch::X86_64 => "x86-64",
            Arch::Arm64 => "arm64",
        }
    }

    pub fn wintun_dir(self) -> &'static str {
        match self {
            Arch::X86_64 => "amd64",
            Arch::Arm64 => "arm64",
        }
    }
}

pub fn detect_os() -> Os {
    if cfg!(target_os = "linux") {
        Os::Linux
    } else if cfg!(target_os = "windows") {
        Os::Windows
    } else if cfg!(target_os = "macos") {
        Os::MacOs
    } else {
        Os::Linux
    }
}

pub fn detect_arch() -> Option<Arch> {
    if cfg!(target_arch = "x86_64") {
        Some(Arch::X86_64)
    } else if cfg!(target_arch = "aarch64") {
        Some(Arch::Arm64)
    } else {
        None
    }
}

pub fn is_elevated() -> bool {
    check_elevated_impl()
}

#[cfg(unix)]
fn check_elevated_impl() -> bool {
    std::process::Command::new("id")
        .arg("-u")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
        .unwrap_or(false)
}

#[cfg(windows)]
fn check_elevated_impl() -> bool {
    std::process::Command::new("net")
        .arg("session")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(not(any(unix, windows)))]
fn check_elevated_impl() -> bool {
    true
}