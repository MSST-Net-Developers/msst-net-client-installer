mod detect;
mod github;
mod install;
mod ui;

use detect::{Arch, LinuxPkgManager, Os};
use install::ControllerType;

enum Operation {
    Install,
    Update,
    Uninstall,
}

fn main() -> anyhow::Result<()> {
    ui::print_banner();

    if !detect::is_elevated() {
        ui::print_error("安装程序必须以 root（Linux/macOS）或管理员（Windows）权限运行。");
        std::process::exit(1);
    }

    let os = confirm_os(detect::detect_os());
    let operation = select_operation();

    if matches!(operation, Operation::Uninstall) {
        return run_uninstall(os);
    }

    let arch = match detect::detect_arch() {
        Some(a) => {
            println!("CPU 架构：{}", a.display_name());
            println!();
            a
        }
        None => {
            eprintln!("错误：不支持的 CPU 架构，仅支持 x86-64 和 arm64。");
            std::process::exit(1);
        }
    };

    match operation {
        Operation::Install => run_install(os, arch),
        Operation::Update => run_update(os, arch),
        Operation::Uninstall => unreachable!(),
    }
}

fn run_install(os: Os, arch: Arch) -> anyhow::Result<()> {
    let controller_type = select_controller();
    let mirror = select_mirror();

    // On Linux, detect the package manager so we can offer native packages.
    let linux_pkg_mgr = if os == Os::Linux {
        let pm = detect::detect_linux_pkg_manager();
        match pm {
            LinuxPkgManager::Deb => println!("检测到包管理器：dpkg（Debian/Ubuntu 系列）"),
            LinuxPkgManager::Rpm => println!("检测到包管理器：rpm（Fedora/RHEL 系列）"),
            LinuxPkgManager::Other => println!("未检测到 deb/rpm 包管理器，将使用 AppImage"),
        }
        println!();
        pm
    } else {
        LinuxPkgManager::Other
    };

    ui::print_section(&format!("从 {} 获取最新版本信息", mirror.display_name()));
    let client = reqwest::blocking::Client::new();
    let release = github::fetch_latest_release(&client, mirror)?;
    ui::print_info(&format!("最新版本：{}", release.tag_name));

    let core_name = build_core_name(os, arch);
    let (controller_name, use_native_pkg) =
        resolve_controller_name(os, arch, controller_type, linux_pkg_mgr, &release);

    let core_asset = release.find_asset(&core_name).ok_or_else(|| {
        anyhow::anyhow!(
            "在 Release 中未找到核心文件 '{}'。可用文件：\n{}",
            core_name,
            release.assets.iter().map(|a| format!("  - {}", a.name)).collect::<Vec<_>>().join("\n")
        )
    })?;
    let controller_asset = release.find_asset(&controller_name).ok_or_else(|| {
        anyhow::anyhow!(
            "在 Release 中未找到控制器文件 '{}'。可用文件：\n{}",
            controller_name,
            release.assets.iter().map(|a| format!("  - {}", a.name)).collect::<Vec<_>>().join("\n")
        )
    })?;

    let install_dir = os.install_dir();
    ui::print_info(&format!("安装目录：{}", install_dir.display()));
    std::fs::create_dir_all(&install_dir)?;

    let core_dest = install_dir.join(&core_name);
    println!("正在下载网络核心（{}）...", format_size(core_asset.size));
    github::download_file(&client, &core_asset.browser_download_url, &core_dest)?;
    #[cfg(unix)]
    install::make_executable(&core_dest)?;

    println!("正在下载控制器（{}）...", format_size(controller_asset.size));
    let controller_dest = if use_native_pkg {
        // Download to /tmp, install via package manager, then delete the package file.
        let tmp_path = std::env::temp_dir().join(&controller_name);
        github::download_file(&client, &controller_asset.browser_download_url, &tmp_path)?;
        install::install_linux_native_package(&tmp_path, linux_pkg_mgr)?;
        let _ = std::fs::remove_file(&tmp_path);
        None
    } else {
        let dest = install_dir.join(&controller_name);
        github::download_file(&client, &controller_asset.browser_download_url, &dest)?;
        #[cfg(unix)]
        install::make_executable(&dest)?;
        // Create wrapper script for AppImage (sets EGL/Wayland env vars).
        if os == Os::Linux && matches!(controller_type, ControllerType::Tauri) {
            install::create_appimage_wrapper(&dest)?;
        }
        Some(dest)
    };

    if os == Os::Windows {
        install::install_wintun(&client, &install_dir, arch)?;
    }

    ui::print_section("配置系统服务");
    install::install_service(os, &core_dest)?;

    println!();
    ui::print_success("安装完成！");
    ui::print_info(&format!("安装目录：{}", install_dir.display()));
    ui::print_info(&format!("网络核心：{}", core_dest.display()));
    if let Some(p) = &controller_dest {
        ui::print_info(&format!("控制器  ：{}", p.display()));
    } else {
        ui::print_info("控制器  ：已通过系统包管理器安装");
    }
    ui::print_info("服务    ：msst-net（已启用并运行）");

    Ok(())
}

fn run_update(os: Os, arch: Arch) -> anyhow::Result<()> {
    let controller_type = select_controller();
    let mirror = select_mirror();

    let linux_pkg_mgr = if os == Os::Linux {
        let pm = detect::detect_linux_pkg_manager();
        match pm {
            LinuxPkgManager::Deb => println!("检测到包管理器：dpkg（Debian/Ubuntu 系列）"),
            LinuxPkgManager::Rpm => println!("检测到包管理器：rpm（Fedora/RHEL 系列）"),
            LinuxPkgManager::Other => println!("未检测到 deb/rpm 包管理器，将使用 AppImage"),
        }
        println!();
        pm
    } else {
        LinuxPkgManager::Other
    };

    ui::print_section(&format!("从 {} 获取最新版本信息", mirror.display_name()));
    let client = reqwest::blocking::Client::new();
    let release = github::fetch_latest_release(&client, mirror)?;
    ui::print_info(&format!("最新版本：{}", release.tag_name));

    let core_name = build_core_name(os, arch);
    let (controller_name, use_native_pkg) =
        resolve_controller_name(os, arch, controller_type, linux_pkg_mgr, &release);

    let core_asset = release.find_asset(&core_name).ok_or_else(|| {
        anyhow::anyhow!("在 Release 中未找到核心文件 '{}'", core_name)
    })?;
    let controller_asset = release.find_asset(&controller_name).ok_or_else(|| {
        anyhow::anyhow!("在 Release 中未找到控制器文件 '{}'", controller_name)
    })?;

    let install_dir = os.install_dir();
    ui::print_info(&format!("安装目录：{}", install_dir.display()));
    std::fs::create_dir_all(&install_dir)?;

    ui::print_section("停止服务");
    install::stop_service(os)?;

    let core_dest = install_dir.join(&core_name);
    println!("正在下载网络核心（{}）...", format_size(core_asset.size));
    github::download_file(&client, &core_asset.browser_download_url, &core_dest)?;
    #[cfg(unix)]
    install::make_executable(&core_dest)?;

    println!("正在下载控制器（{}）...", format_size(controller_asset.size));
    let controller_dest = if use_native_pkg {
        let tmp_path = std::env::temp_dir().join(&controller_name);
        github::download_file(&client, &controller_asset.browser_download_url, &tmp_path)?;
        install::install_linux_native_package(&tmp_path, linux_pkg_mgr)?;
        let _ = std::fs::remove_file(&tmp_path);
        None
    } else {
        let dest = install_dir.join(&controller_name);
        github::download_file(&client, &controller_asset.browser_download_url, &dest)?;
        #[cfg(unix)]
        install::make_executable(&dest)?;
        if os == Os::Linux && matches!(controller_type, ControllerType::Tauri) {
            install::create_appimage_wrapper(&dest)?;
        }
        Some(dest)
    };

    if os == Os::Windows {
        install::install_wintun(&client, &install_dir, arch)?;
    }

    ui::print_section("重启服务");
    install::restart_service(os)?;

    println!();
    ui::print_success("更新完成！");
    ui::print_info(&format!("安装目录：{}", install_dir.display()));
    ui::print_info(&format!("网络核心：{}", core_dest.display()));
    if let Some(p) = &controller_dest {
        ui::print_info(&format!("控制器  ：{}", p.display()));
    } else {
        ui::print_info("控制器  ：已通过系统包管理器安装");
    }

    Ok(())
}

fn run_uninstall(os: Os) -> anyhow::Result<()> {
    ui::print_warning("此操作将停止并删除 MSST-Net 服务及所有已安装文件。");
    if !ui::prompt_yn("确认卸载？") {
        ui::print_info("已取消。");
        return Ok(());
    }
    println!();

    ui::print_section("停止并移除服务");
    install::uninstall_service(os)?;

    // Remove the AppImage wrapper script if present.
    if os == Os::Linux {
        install::remove_appimage_wrapper();
    }

    let install_dir = os.install_dir();
    if install_dir.exists() {
        ui::print_info(&format!("正在删除安装目录：{}", install_dir.display()));
        std::fs::remove_dir_all(&install_dir)?;
    }

    println!();
    ui::print_success("卸载完成！");

    Ok(())
}

fn confirm_os(detected: Os) -> Os {
    println!("检测到操作系统：{}", detected.display_name());
    if ui::prompt_yn("是否正确？") {
        println!();
        return detected;
    }
    println!();
    let idx = ui::prompt_select("请选择操作系统：", &["Linux", "Windows", "macOS"]);
    println!();
    match idx {
        0 => Os::Linux,
        1 => Os::Windows,
        2 => Os::MacOs,
        _ => unreachable!(),
    }
}

fn select_operation() -> Operation {
    let idx = ui::prompt_select(
        "请选择操作：",
        &["安装", "更新", "卸载"],
    );
    println!();
    match idx {
        0 => Operation::Install,
        1 => Operation::Update,
        2 => Operation::Uninstall,
        _ => unreachable!(),
    }
}

fn select_mirror() -> github::Mirror {
    let idx = ui::prompt_select(
        "请选择下载源：",
        &["Gitee（国内镜像，推荐）", "GitHub（官方源）"],
    );
    println!();
    match idx {
        0 => github::Mirror::Gitee,
        1 => github::Mirror::GitHub,
        _ => unreachable!(),
    }
}

fn select_controller() -> ControllerType {
    let idx = ui::prompt_select(
        "请选择要安装的控制器类型：",
        &[
            "桌面版（Tauri）— 原生 GUI 应用",
            "Web UI         — 浏览器界面",
            "CLI            — 命令行界面",
        ],
    );
    println!();
    match idx {
        0 => ControllerType::Tauri,
        1 => ControllerType::WebUi,
        2 => ControllerType::Cli,
        _ => unreachable!(),
    }
}

fn build_core_name(os: Os, arch: Arch) -> String {
    format!(
        "msst-net-client-core-{}-{}{}",
        os.platform_str(),
        arch.core_arch_str(),
        os.exe_suffix()
    )
}

/// Determine the controller asset filename.
///
/// For Linux Tauri, we first try the native package format (deb/rpm).  If the
/// release does not contain that asset (e.g. because only the AppImage was
/// published for this release), we transparently fall back to `.AppImage`.
///
/// Returns `(asset_name, use_native_pkg)` where `use_native_pkg` is `true`
/// when a deb/rpm asset was found and should be installed via the OS package
/// manager.
fn resolve_controller_name(
    os: Os,
    arch: Arch,
    controller_type: ControllerType,
    linux_pkg_mgr: LinuxPkgManager,
    release: &github::ReleaseInfo,
) -> (String, bool) {
    let base = format!(
        "msst-net-client-controller-{}-{}-{}",
        controller_type.type_str(),
        os.platform_str(),
        arch.arch_str(),
    );

    // On Linux, for the Tauri controller, try native package first.
    if os == Os::Linux && matches!(controller_type, ControllerType::Tauri) {
        let preferred = ControllerType::linux_preferred_suffix(linux_pkg_mgr);
        if preferred != ".AppImage" {
            let native_name = format!("{}{}", base, preferred);
            if release.find_asset(&native_name).is_some() {
                println!(
                    "找到原生 {} 包，将使用系统包管理器安装。",
                    preferred.trim_start_matches('.')
                );
                return (native_name, true);
            }
            println!(
                "未在 Release 中找到 {} 包，退回到 AppImage。",
                preferred.trim_start_matches('.')
            );
        }
    }

    let suffix = controller_type.os_suffix(os);
    (format!("{}{}", base, suffix), false)
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}