use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;
use std::io::{Read, Write};
use std::path::Path;

const GITHUB_API: &str = "https://api.github.com/repos/MSST-Net-Developers/msst-net-client-release/releases/latest";
const GITEE_API: &str = "https://gitee.com/api/v5/repos/Abjust/msst-net-client-release/releases/latest";
const USER_AGENT: &str = concat!("msst-net-client-installer/", env!("CARGO_PKG_VERSION"));

#[derive(Clone, Copy, PartialEq)]
pub enum Mirror {
    GitHub,
    Gitee,
}

impl Mirror {
    pub fn display_name(self) -> &'static str {
        match self {
            Mirror::GitHub => "GitHub（官方源）",
            Mirror::Gitee => "Gitee（国内镜像）",
        }
    }
}

#[derive(Deserialize)]
pub struct ReleaseInfo {
    pub tag_name: String,
    pub assets: Vec<Asset>,
}

#[derive(Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
    #[serde(default)]
    pub size: u64,
}

impl ReleaseInfo {
    pub fn find_asset(&self, name: &str) -> Option<&Asset> {
        self.assets.iter().find(|a| a.name == name)
    }
}

pub fn fetch_latest_release(
    client: &reqwest::blocking::Client,
    mirror: Mirror,
) -> Result<ReleaseInfo> {
    let mut builder = client
        .get(match mirror {
            Mirror::GitHub => GITHUB_API,
            Mirror::Gitee => GITEE_API,
        })
        .header("User-Agent", USER_AGENT);

    if matches!(mirror, Mirror::GitHub) {
        builder = builder.header("Accept", "application/vnd.github+json");
    }

    let release = builder
        .send()?
        .error_for_status()?
        .json::<ReleaseInfo>()?;
    Ok(release)
}

pub fn download_file(client: &reqwest::blocking::Client, url: &str, dest: &Path) -> Result<()> {
    let mut response = client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .send()?
        .error_for_status()?;

    let total = response.content_length().unwrap_or(0);

    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("  [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("=>-"),
    );

    let mut file = std::fs::File::create(dest)?;
    let mut buf = vec![0u8; 65536];
    loop {
        let n = response.read(&mut buf)?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        pb.inc(n as u64);
    }
    pb.finish_with_message("done");

    Ok(())
}

pub fn download_bytes(client: &reqwest::blocking::Client, url: &str) -> Result<Vec<u8>> {
    let bytes = client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .send()?
        .error_for_status()?
        .bytes()?;
    Ok(bytes.to_vec())
}