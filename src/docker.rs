use crate::{detect::Os, ui};
use anyhow::Result;
use std::path::{Path, PathBuf};

const COMPOSE_TEMPLATE: &str = include_str!("../docker/docker-compose.yml");
const DOCKER_DIR: &str = "/opt/msst-net-docker";

#[derive(Clone, Copy)]
enum ComposeCmd {
    V2,
    V1,
}

fn detect_compose() -> Result<ComposeCmd> {
    let docker_ok = std::process::Command::new("docker")
        .arg("info")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !docker_ok {
        anyhow::bail!("未检测到运行中的 Docker Engine，请先安装并启动 Docker。");
    }

    if std::process::Command::new("docker")
        .args(["compose", "version"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        return Ok(ComposeCmd::V2);
    }
    if std::process::Command::new("docker-compose")
        .arg("version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        return Ok(ComposeCmd::V1);
    }
    anyhow::bail!("未找到 docker compose 或 docker-compose，请安装 Docker Compose。");
}

fn run_compose(compose_cmd: ComposeCmd, dir: &Path, args: &[&str]) -> Result<()> {
    let status = match compose_cmd {
        ComposeCmd::V2 => std::process::Command::new("docker")
            .current_dir(dir)
            .arg("compose")
            .args(args)
            .status()?,
        ComposeCmd::V1 => std::process::Command::new("docker-compose")
            .current_dir(dir)
            .args(args)
            .status()?,
    };
    if !status.success() {
        anyhow::bail!("docker compose 命令失败，退出码：{:?}", status.code());
    }
    Ok(())
}

fn deploy_dir() -> PathBuf {
    PathBuf::from(DOCKER_DIR)
}

fn write_compose(dir: &Path, admin_password: &str) -> Result<()> {
    let content = COMPOSE_TEMPLATE.replace("changeme", admin_password);
    std::fs::write(dir.join("docker-compose.yml"), content)?;
    Ok(())
}

pub fn run_docker_install(os: Os) -> Result<()> {
    if os != Os::Linux {
        anyhow::bail!(
            "Docker 安装模式仅支持 Linux（容器需要 NET_ADMIN 权限和 /dev/net/tun 设备）。"
        );
    }

    let compose_cmd = detect_compose()?;
    let label = match compose_cmd {
        ComposeCmd::V2 => "docker compose（v2）",
        ComposeCmd::V1 => "docker-compose（v1）",
    };
    println!("检测到 Docker Compose：{}", label);
    println!();

    let raw = ui::prompt_password("请设置 WebUI 管理员密码（直接回车使用默认值 changeme）");
    let password = if raw.is_empty() {
        "changeme".to_string()
    } else {
        raw
    };

    let dir = deploy_dir();
    std::fs::create_dir_all(dir.join("data"))?;
    std::fs::create_dir_all(dir.join("run"))?;

    write_compose(&dir, &password)?;
    ui::print_info(&format!("Compose 文件已写入：{}/docker-compose.yml", DOCKER_DIR));

    ui::print_section("拉取 Docker 镜像");
    run_compose(compose_cmd, &dir, &["pull"])?;

    ui::print_section("启动服务");
    run_compose(compose_cmd, &dir, &["up", "-d"])?;

    println!();
    ui::print_success("Docker 安装完成！");
    ui::print_info(&format!("部署目录：{}", dir.display()));
    ui::print_info("WebUI 地址：http://localhost:18080");
    if password == "changeme" {
        ui::print_warning("使用了默认密码 changeme，建议登录后立即修改。");
    }

    Ok(())
}

pub fn run_docker_update(os: Os) -> Result<()> {
    if os != Os::Linux {
        anyhow::bail!("Docker 安装模式仅支持 Linux。");
    }

    let dir = deploy_dir();
    if !dir.join("docker-compose.yml").exists() {
        anyhow::bail!(
            "未找到 Docker 部署目录 {}，请先执行 Docker 安装。",
            DOCKER_DIR
        );
    }

    let compose_cmd = detect_compose()?;

    ui::print_section("拉取最新镜像");
    run_compose(compose_cmd, &dir, &["pull"])?;

    ui::print_section("重启服务");
    run_compose(compose_cmd, &dir, &["up", "-d"])?;

    println!();
    ui::print_success("Docker 更新完成！");

    Ok(())
}

pub fn run_docker_uninstall(os: Os) -> Result<()> {
    if os != Os::Linux {
        anyhow::bail!("Docker 安装模式仅支持 Linux。");
    }

    ui::print_warning("此操作将停止并删除所有 MSST-Net 容器和镜像，数据目录将保留。");
    if !ui::prompt_yn("确认卸载？") {
        ui::print_info("已取消。");
        return Ok(());
    }
    println!();

    let dir = deploy_dir();
    if dir.join("docker-compose.yml").exists() {
        let compose_cmd = detect_compose()?;
        ui::print_section("停止并移除容器");
        let _ = run_compose(compose_cmd, &dir, &["down", "--rmi", "all"]);
        let _ = std::fs::remove_file(dir.join("docker-compose.yml"));
    }

    println!();
    ui::print_success("Docker 卸载完成！");
    ui::print_info(&format!(
        "数据目录 {}/data 已保留，如需彻底清除请手动删除。",
        DOCKER_DIR
    ));

    Ok(())
}
