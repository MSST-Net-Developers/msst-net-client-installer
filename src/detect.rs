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
    pub fn display_name(self) -> &'static str {
        match self {
            Arch::X86_64 => "x86-64",
            Arch::Arm64 => "arm64",
        }
    }

    pub fn arch_str(self) -> &'static str {
        match self {
            Arch::X86_64 => "x86_64",
            Arch::Arm64 => "arm64",
        }
    }

    pub fn core_arch_str(self) -> &'static str {
        match self {
            Arch::X86_64 => "x86_64",
            Arch::Arm64 => "aarch64",
        }
    }

    pub fn wintun_dir(self) -> &'static str {
        match self {
            Arch::X86_64 => "amd64",
            Arch::Arm64 => "arm64",
        }
    }
}

/// Which native package manager is available on this Linux system.
/// Used to decide whether to install a .deb / .rpm instead of the AppImage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LinuxPkgManager {
    /// Debian / Ubuntu / Mint / Pop!_OS …  (dpkg + apt)
    Deb,
    /// Fedora / RHEL / CentOS / openSUSE … (rpm + dnf/yum/zypper)
    Rpm,
    /// Arch / Void / other — fall back to AppImage
    Other,
}

/// Detect the Linux package manager by checking well-known marker files and
/// then falling back to probing the `dpkg` / `rpm` binaries with `which`.
pub fn detect_linux_pkg_manager() -> LinuxPkgManager {
    // File-based detection is reliable and fast.
    if std::path::Path::new("/etc/debian_version").exists() {
        return LinuxPkgManager::Deb;
    }
    for marker in &[
        "/etc/fedora-release",
        "/etc/redhat-release",
        "/etc/centos-release",
        "/etc/SuSE-release",
        "/etc/opensuse-release",
    ] {
        if std::path::Path::new(marker).exists() {
            return LinuxPkgManager::Rpm;
        }
    }
    // Fall back to probing binaries.
    if which_on_path("dpkg") {
        return LinuxPkgManager::Deb;
    }
    if which_on_path("rpm") {
        return LinuxPkgManager::Rpm;
    }
    LinuxPkgManager::Other
}

fn which_on_path(cmd: &str) -> bool {
    std::process::Command::new("sh")
        .args(["-c", &format!("command -v {cmd}")])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
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