use anyhow::{bail, Context, Result};
use flate2::read::GzDecoder;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use tar::Archive;

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
struct LatestResponse {
    version: String,
    platforms: HashMap<String, PlatformInfo>,
}

#[derive(Deserialize)]
struct PlatformInfo {
    sha256: String,
    #[allow(dead_code)]
    size_bytes: u64,
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

async fn fetch_latest_release(api_url: &str) -> Result<LatestResponse> {
    let url = format!("{api_url}/cli/latest");
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("User-Agent", "procrast")
        .send()
        .await
        .context("Failed to reach update server")?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        bail!("Update server returned {status}: {body}");
    }

    resp.json()
        .await
        .context("Failed to parse release info")
}

async fn download_and_replace(api_url: &str, version: &str, expected_sha256: &str) -> Result<()> {
    let url = format!("{api_url}/cli/download/{version}/{TARGET}");

    println!("Downloading v{version} for {TARGET}...");

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("User-Agent", "procrast")
        .send()
        .await
        .context("Failed to download release")?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        bail!("Download failed ({status}): {body}");
    }

    let bytes = resp
        .bytes()
        .await
        .context("Failed to read download")?;

    // Verify SHA256
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let actual_sha256 = format!("{:x}", hasher.finalize());
    if actual_sha256 != expected_sha256 {
        bail!(
            "SHA256 mismatch: expected {expected_sha256}, got {actual_sha256}. Download may be corrupted."
        );
    }

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
pub async fn check_latest_version(api_url: &str) -> Result<String> {
    // Use cache if less than 1 hour old
    if let Some((version, modified)) = read_cached_version() {
        if let Ok(elapsed) = modified.elapsed() {
            if elapsed.as_secs() < 3600 {
                return Ok(version);
            }
        }
    }

    let release = fetch_latest_release(api_url).await?;
    write_cached_version(&release.version);
    Ok(release.version)
}

pub async fn update(api_url: &str, check_only: bool, json: bool) -> Result<()> {
    let release = fetch_latest_release(api_url).await?;
    let latest = &release.version;
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

    let platform_info = release
        .platforms
        .get(TARGET)
        .context(format!("No release available for {TARGET}"))?;

    download_and_replace(api_url, latest, &platform_info.sha256).await?;
    crate::auth::delete_token()?;
    println!("Updated to v{latest}. Please run `procrast login` to re-authenticate.");
    Ok(())
}
