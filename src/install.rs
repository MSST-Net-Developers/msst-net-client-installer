use crate::detect::{Arch, LinuxPkgManager, Os};
use anyhow::Result;
use std::path::Path;

const SYSTEMD_UNIT: &str = "/etc/systemd/system/msst-net.service";
const LAUNCHD_PLIST: &str = "/Library/LaunchDaemons/net.msst.client.plist";
/// Wrapper script placed on $PATH so users can launch the Tauri controller directly.
const APPIMAGE_WRAPPER: &str = "/usr/local/bin/msst-net-tauri";

#[derive(Debug, Clone, Copy)]
pub enum ControllerType {
    Tauri,
    WebUi,
    Cli,
}

impl ControllerType {
    pub fn type_str(self) -> &'static str {
        match self {
            ControllerType::Tauri => "tauri",
            ControllerType::WebUi => "webui",
            ControllerType::Cli => "cli",
        }
    }

    pub fn os_suffix(self, os: crate::detect::Os) -> &'static str {
        match (self, os) {
            (ControllerType::Tauri, crate::detect::Os::Linux) => ".AppImage",
            (ControllerType::Tauri, crate::detect::Os::MacOs) => ".dmg",
            _ => os.exe_suffix(),
        }
    }

    /// Return the preferred Linux Tauri asset suffix given the detected package
    /// manager.  The caller should try this first; if the asset is not found in
    /// the release it should fall back to `os_suffix` (AppImage).
    pub fn linux_preferred_suffix(pkg_mgr: LinuxPkgManager) -> &'static str {
        match pkg_mgr {
            LinuxPkgManager::Deb => ".deb",
            LinuxPkgManager::Rpm => ".rpm",
            LinuxPkgManager::Other => ".AppImage",
        }
    }
}

/// Install a downloaded .deb or .rpm package using the system package manager.
/// `path` is the downloaded package file (may be in /tmp).
pub fn install_linux_native_package(path: &Path, pkg_mgr: LinuxPkgManager) -> Result<()> {
    let path_str = path.to_string_lossy();
    match pkg_mgr {
        LinuxPkgManager::Deb => {
            println!("正在使用 dpkg 安装 deb 包...");
            run_cmd("dpkg", &["-i", &path_str])?;
            println!("deb 包安装完成。");
        }
        LinuxPkgManager::Rpm => {
            // Prefer dnf → yum → rpm for better dependency resolution.
            println!("正在使用 rpm 安装 rpm 包...");
            let installed = try_rpm_install(&path_str);
            if !installed {
                run_cmd("rpm", &["-Uvh", "--force", &path_str])?;
            }
            println!("rpm 包安装完成。");
        }
        LinuxPkgManager::Other => {}
    }
    Ok(())
}

fn try_rpm_install(path: &str) -> bool {
    for pm in &["dnf", "yum", "zypper"] {
        let args: Vec<&str> = if *pm == "zypper" {
            vec!["install", "--no-confirm", path]
        } else {
            vec!["install", "-y", path]
        };
        if std::process::Command::new(pm)
            .args(&args)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return true;
        }
    }
    false
}

/// Create a wrapper shell script at `/usr/local/bin/msst-net-tauri` that sets
/// the environment variables required to work around WebKitGTK EGL issues on
/// Wayland (KDE Plasma, GNOME Wayland, etc.) and then executes the AppImage.
pub fn create_appimage_wrapper(appimage_path: &Path) -> Result<()> {
    let script = format!(
        "#!/bin/sh\n\
         # MSST-Net Tauri controller launcher\n\
         # Sets WebKitGTK / EGL workarounds for Wayland environments.\n\
         export WEBKIT_DISABLE_COMPOSITING_MODE=\"${{WEBKIT_DISABLE_COMPOSITING_MODE:-1}}\"\n\
         export WEBKIT_DISABLE_DMABUF_RENDERER=\"${{WEBKIT_DISABLE_DMABUF_RENDERER:-1}}\"\n\
         if [ -n \"$WAYLAND_DISPLAY\" ] && [ -z \"$GDK_BACKEND\" ]; then\n\
             export GDK_BACKEND=x11\n\
         fi\n\
         exec {} \"$@\"\n",
        appimage_path.display()
    );
    std::fs::write(APPIMAGE_WRAPPER, script)?;
    #[cfg(unix)]
    make_executable(Path::new(APPIMAGE_WRAPPER))?;
    println!("已创建启动脚本：{}", APPIMAGE_WRAPPER);
    Ok(())
}

/// Remove the AppImage wrapper script (used during uninstall).
pub fn remove_appimage_wrapper() {
    let _ = std::fs::remove_file(APPIMAGE_WRAPPER);
}

#[cfg(unix)]
pub fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

pub fn install_wintun(
    client: &reqwest::blocking::Client,
    install_dir: &Path,
    arch: Arch,
) -> Result<()> {
    const WINTUN_URL: &str = "https://www.wintun.net/builds/wintun-0.14.1.zip";

    println!("正在下载 Wintun...");
    let bytes = crate::github::download_bytes(client, WINTUN_URL)?;

    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)?;

    let inner_path = format!("wintun/bin/{}/wintun.dll", arch.wintun_dir());
    let mut entry = archive
        .by_name(&inner_path)
        .map_err(|e| anyhow::anyhow!("在压缩包中未找到 wintun.dll（{}）：{}", inner_path, e))?;

    let dest = install_dir.join("wintun.dll");
    let mut file = std::fs::File::create(&dest)?;
    std::io::copy(&mut entry, &mut file)?;

    println!("Wintun 已安装至：{}", dest.display());
    Ok(())
}

pub fn install_service(os: Os, core_path: &Path) -> Result<()> {
    match os {
        Os::Linux => install_systemd(core_path),
        Os::MacOs => install_launchd(core_path),
        Os::Windows => install_windows_service(core_path),
    }
}

pub fn stop_service(os: Os) -> Result<()> {
    match os {
        Os::Linux => {
            let _ = run_cmd("systemctl", &["stop", "msst-net"]);
        }
        Os::MacOs => {
            let _ = run_cmd("launchctl", &["unload", LAUNCHD_PLIST]);
        }
        Os::Windows => {
            let _ = std::process::Command::new("sc")
                .args(["stop", "msst-net"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
        }
    }
    Ok(())
}

pub fn restart_service(os: Os) -> Result<()> {
    match os {
        Os::Linux => {
            run_cmd("systemctl", &["restart", "msst-net"])?;
            println!("Systemd 服务 'msst-net' 已重启。");
        }
        Os::MacOs => {
            let _ = run_cmd("launchctl", &["unload", LAUNCHD_PLIST]);
            run_cmd("launchctl", &["load", "-w", LAUNCHD_PLIST])?;
            println!("LaunchDaemon 'net.msst.client' 已重新加载。");
        }
        Os::Windows => {
            run_cmd("sc", &["start", "msst-net"])?;
            println!("Windows 服务 'msst-net' 已启动。");
        }
    }
    Ok(())
}

pub fn uninstall_service(os: Os) -> Result<()> {
    match os {
        Os::Linux => {
            let _ = run_cmd("systemctl", &["stop", "msst-net"]);
            let _ = run_cmd("systemctl", &["disable", "msst-net"]);
            let _ = std::fs::remove_file(SYSTEMD_UNIT);
            let _ = run_cmd("systemctl", &["daemon-reload"]);
            println!("Systemd 服务 'msst-net' 已移除。");
        }
        Os::MacOs => {
            let _ = run_cmd("launchctl", &["unload", LAUNCHD_PLIST]);
            let _ = std::fs::remove_file(LAUNCHD_PLIST);
            println!("LaunchDaemon 'net.msst.client' 已移除。");
        }
        Os::Windows => {
            let _ = std::process::Command::new("sc")
                .args(["stop", "msst-net"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
            let _ = std::process::Command::new("sc")
                .args(["delete", "msst-net"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
            println!("Windows 服务 'msst-net' 已移除。");
        }
    }
    Ok(())
}

fn install_systemd(core_path: &Path) -> Result<()> {
    let service = format!(
        "[Unit]\n\
         Description=MSST-Net Client Core\n\
         After=network.target\n\
         \n\
         [Service]\n\
         Type=simple\n\
         ExecStart={}\n\
         Restart=always\n\
         RestartSec=5\n\
         \n\
         [Install]\n\
         WantedBy=multi-user.target\n",
        core_path.display()
    );

    std::fs::write(SYSTEMD_UNIT, service)?;

    run_cmd("systemctl", &["daemon-reload"])?;
    run_cmd("systemctl", &["enable", "msst-net"])?;
    run_cmd("systemctl", &["start", "msst-net"])?;

    println!("Systemd 服务 'msst-net' 已启用并启动。");
    Ok(())
}

fn install_launchd(core_path: &Path) -> Result<()> {
    let plist = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \
         \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n\
         <dict>\n\
             <key>Label</key>\n\
             <string>net.msst.client</string>\n\
             <key>ProgramArguments</key>\n\
             <array>\n\
                 <string>{}</string>\n\
             </array>\n\
             <key>RunAtLoad</key>\n\
             <true/>\n\
             <key>KeepAlive</key>\n\
             <true/>\n\
         </dict>\n\
         </plist>\n",
        core_path.display()
    );

    std::fs::write(LAUNCHD_PLIST, plist)?;
    run_cmd("launchctl", &["load", "-w", LAUNCHD_PLIST])?;

    println!("LaunchDaemon 'net.msst.client' 已加载。");
    Ok(())
}

fn install_windows_service(core_path: &Path) -> Result<()> {
    // Remove existing service if present (ignore errors)
    let _ = std::process::Command::new("sc")
        .args(["stop", "msst-net"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    let _ = std::process::Command::new("sc")
        .args(["delete", "msst-net"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    let bin_path_arg = format!("binPath= \"{}\"", core_path.display());
    run_cmd(
        "sc",
        &[
            "create",
            "msst-net",
            &bin_path_arg,
            "start= auto",
            "DisplayName= MSST-Net Client",
        ],
    )?;
    run_cmd(
        "sc",
        &["description", "msst-net", "MSST-Net Client Core Network Service"],
    )?;
    run_cmd("sc", &["start", "msst-net"])?;

    println!("Windows 服务 'msst-net' 已创建并启动。");
    Ok(())
}

fn run_cmd(prog: &str, args: &[&str]) -> Result<()> {
    let status = std::process::Command::new(prog)
        .args(args)
        .status()
        .map_err(|e| anyhow::anyhow!("无法执行 '{}'：{}", prog, e))?;

    if !status.success() {
        anyhow::bail!("命令 '{}' 退出码：{:?}", prog, status.code());
    }
    Ok(())
}