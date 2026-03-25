use anyhow::{bail, Context, Result};
use flate2::read::GzDecoder;
use serde::Deserialize;
use std::env;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use tar::Archive;

const GITHUB_REPO: &str = "yelsed/procrast-cli";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const TARGET: &str = "aarch64-apple-darwin";
#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
const TARGET: &str = "x86_64-apple-darwin";
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const TARGET: &str = "x86_64-unknown-linux-gnu";
#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
const TARGET: &str = "x86_64-pc-windows-msvc";

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

#[derive(serde::Serialize)]
struct UpdateCheckResult {
    current: String,
    latest: String,
    update_available: bool,
}

fn parse_version(v: &str) -> (u32, u32, u32) {
    let v = v.strip_prefix('v').unwrap_or(v);
    let parts: Vec<u32> = v.split('.').filter_map(|s| s.parse().ok()).collect();
    (
        parts.first().copied().unwrap_or(0),
        parts.get(1).copied().unwrap_or(0),
        parts.get(2).copied().unwrap_or(0),
    )
}

fn is_newer(latest: &str, current: &str) -> bool {
    let l = parse_version(latest);
    let c = parse_version(current);
    l > c
}

async fn fetch_latest_release() -> Result<Release> {
    let url = format!("https://api.github.com/repos/{GITHUB_REPO}/releases/latest");
    let client = reqwest::Client::new();
    let release: Release = client
        .get(&url)
        .header("User-Agent", "procrast-cli")
        .send()
        .await
        .context("Failed to reach GitHub API")?
        .json()
        .await
        .context("Failed to parse release info")?;
    Ok(release)
}

async fn download_and_replace(release: &Release) -> Result<()> {
    let asset_name = format!("procrast-cli-{TARGET}.tar.gz");
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .context(format!("No release asset found for {TARGET}"))?;

    println!("Downloading {}...", asset.name);

    let client = reqwest::Client::new();
    let bytes = client
        .get(&asset.browser_download_url)
        .header("User-Agent", "procrast-cli")
        .send()
        .await
        .context("Failed to download release")?
        .bytes()
        .await
        .context("Failed to read download")?;

    let current_exe = env::current_exe().context("Failed to determine current binary path")?;
    let _parent = current_exe.parent().context("No parent directory")?;

    let decoder = GzDecoder::new(&bytes[..]);
    let mut archive = Archive::new(decoder);

    let mut found = false;
    for entry in archive.entries().context("Failed to read tar archive")? {
        let mut entry = entry.context("Failed to read tar entry")?;
        let path = entry.path().context("Failed to read entry path")?;
        let filename = path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or_default();

        if filename == "procrast-cli" || filename == "procrast" {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;

            let backup = current_exe.with_extension("old");
            fs::rename(&current_exe, &backup)
                .context("Failed to back up current binary")?;

            if let Err(e) = fs::write(&current_exe, &buf) {
                // Restore backup on failure
                let _ = fs::rename(&backup, &current_exe);
                bail!("Failed to write new binary: {e}");
            }

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&current_exe, fs::Permissions::from_mode(0o755))?;
            }

            let _ = fs::remove_file(&backup);
            found = true;
            break;
        }
    }

    if !found {
        bail!("Binary not found in downloaded archive");
    }

    Ok(())
}

fn cache_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cache")
        .join("procrast-cli")
}

fn version_cache_path() -> PathBuf {
    cache_dir().join("latest_version")
}

#[allow(dead_code)]
fn read_cached_version() -> Option<(String, std::time::SystemTime)> {
    let path = version_cache_path();
    let metadata = fs::metadata(&path).ok()?;
    let modified = metadata.modified().ok()?;
    let content = fs::read_to_string(&path).ok()?;
    Some((content.trim().to_string(), modified))
}

fn write_cached_version(version: &str) {
    let path = version_cache_path();
    let _ = fs::create_dir_all(path.parent().unwrap());
    let _ = fs::write(path, version);
}

#[allow(dead_code)]
pub async fn check_latest_version() -> Result<String> {
    // Use cache if less than 1 hour old
    if let Some((version, modified)) = read_cached_version() {
        if let Ok(elapsed) = modified.elapsed() {
            if elapsed.as_secs() < 3600 {
                return Ok(version);
            }
        }
    }

    let release = fetch_latest_release().await?;
    let version = release.tag_name.strip_prefix('v').unwrap_or(&release.tag_name).to_string();
    write_cached_version(&version);
    Ok(version)
}

pub async fn update(check_only: bool, json: bool) -> Result<()> {
    let release = fetch_latest_release().await?;
    let latest = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name);
    let update_available = is_newer(latest, CURRENT_VERSION);

    write_cached_version(latest);

    if json {
        let result = UpdateCheckResult {
            current: CURRENT_VERSION.to_string(),
            latest: latest.to_string(),
            update_available,
        };
        println!("{}", serde_json::to_string(&result)?);
        return Ok(());
    }

    if !update_available {
        println!("Already up to date (v{CURRENT_VERSION}).");
        return Ok(());
    }

    println!("Update available: v{CURRENT_VERSION} → v{latest}");

    if check_only {
        println!("Run `procrast update` to install.");
        return Ok(());
    }

    download_and_replace(&release).await?;
    crate::auth::delete_token()?;
    println!("Updated to v{latest}. Please run `procrast login` to re-authenticate.");
    Ok(())
}
