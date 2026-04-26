mod detect;
mod github;
mod install;
mod ui;

use detect::Os;
use install::ControllerType;

fn main() -> anyhow::Result<()> {
    println!("=== MSST-Net 客户端安装程序 ===");
    println!();

    if !detect::is_elevated() {
        eprintln!("错误：安装程序必须以 root（Linux/macOS）或管理员（Windows）权限运行。");
        std::process::exit(1);
    }

    let os = confirm_os(detect::detect_os());

    let arch = match detect::detect_arch() {
        Some(a) => {
            println!("CPU 架构：{}", a.arch_str());
            println!();
            a
        }
        None => {
            eprintln!("错误：不支持的 CPU 架构，仅支持 x86-64 和 arm64。");
            std::process::exit(1);
        }
    };

    let controller_type = select_controller();

    println!();
    println!("正在获取最新版本信息...");
    let client = reqwest::blocking::Client::new();
    let release = github::fetch_latest_release(&client)?;
    println!("最新版本：{}", release.tag_name);
    println!();

    let core_name = format!(
        "msst-net-client-core-{}-{}{}",
        os.platform_str(),
        arch.arch_str(),
        os.exe_suffix()
    );
    let controller_name = format!(
        "msst-net-client-controller-{}-{}-{}{}",
        controller_type.type_str(),
        os.platform_str(),
        arch.arch_str(),
        os.exe_suffix()
    );

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
    println!("安装目录：{}", install_dir.display());
    std::fs::create_dir_all(&install_dir)?;

    let core_dest = install_dir.join(&core_name);
    println!("正在下载网络核心（{}）...", format_size(core_asset.size));
    github::download_file(&client, &core_asset.browser_download_url, &core_dest)?;
    #[cfg(unix)]
    install::make_executable(&core_dest)?;

    let controller_dest = install_dir.join(&controller_name);
    println!("正在下载控制器（{}）...", format_size(controller_asset.size));
    github::download_file(&client, &controller_asset.browser_download_url, &controller_dest)?;
    #[cfg(unix)]
    install::make_executable(&controller_dest)?;

    if os == Os::Windows {
        install::install_wintun(&client, &install_dir, arch)?;
    }

    println!();
    println!("正在配置系统服务...");
    install::install_service(os, &core_dest)?;

    println!();
    println!("=== 安装完成！===");
    println!("安装目录：{}", install_dir.display());
    println!("网络核心：{}", core_dest.display());
    println!("控制器  ：{}", controller_dest.display());
    println!("服务    ：msst-net（已启用并运行）");

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

fn format_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}