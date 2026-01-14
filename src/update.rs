//! Self-update functionality for pte.
//!
//! Downloads and installs the latest version from GitHub releases.

use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use ureq::serde_json;

/// GitHub repository for releases (change this if you fork the project)
const GITHUB_REPO: &str = "j-labbe/pretty-table-explorer";

/// Current version from Cargo.toml
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// GitHub API response for a release
#[derive(Debug)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

/// GitHub release asset
#[derive(Debug)]
struct Asset {
    name: String,
    browser_download_url: String,
}

/// Get the asset name for the current platform
fn get_platform_asset_name() -> Result<String, String> {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;

    match (os, arch) {
        ("linux", "x86_64") => Ok("pte-linux-x86_64".to_string()),
        ("linux", "aarch64") => Ok("pte-linux-aarch64".to_string()),
        ("macos", "x86_64") => Ok("pte-macos-x86_64".to_string()),
        ("macos", "aarch64") => Ok("pte-macos-aarch64".to_string()),
        _ => Err(format!(
            "Unsupported platform: {} {}. Self-update is only available for Linux and macOS on x86_64/aarch64.",
            os, arch
        )),
    }
}

/// Parse version string to comparable tuple (major, minor, patch)
fn parse_version(version: &str) -> Option<(u32, u32, u32)> {
    let v = version.trim_start_matches('v');
    let parts: Vec<&str> = v.split('.').collect();
    if parts.len() >= 3 {
        let major = parts[0].parse().ok()?;
        let minor = parts[1].parse().ok()?;
        let patch = parts[2].parse().ok()?;
        Some((major, minor, patch))
    } else {
        None
    }
}

/// Compare two versions, returns true if new_version > current_version
fn is_newer_version(current: &str, new_version: &str) -> bool {
    match (parse_version(current), parse_version(new_version)) {
        (Some(curr), Some(new)) => new > curr,
        _ => false,
    }
}

/// Fetch the latest release information from GitHub
fn fetch_latest_release() -> Result<Release, String> {
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    );

    let response = ureq::get(&url)
        .set("User-Agent", "pte-self-updater")
        .set("Accept", "application/vnd.github.v3+json")
        .call()
        .map_err(|e| format!("Failed to fetch release info: {}", e))?;

    let json: serde_json::Value = response
        .into_json()
        .map_err(|e| format!("Failed to parse release JSON: {}", e))?;

    let tag_name = json["tag_name"]
        .as_str()
        .ok_or("Missing tag_name in release")?
        .to_string();

    let assets = json["assets"]
        .as_array()
        .ok_or("Missing assets in release")?
        .iter()
        .filter_map(|asset| {
            let name = asset["name"].as_str()?.to_string();
            let browser_download_url = asset["browser_download_url"].as_str()?.to_string();
            Some(Asset {
                name,
                browser_download_url,
            })
        })
        .collect();

    Ok(Release { tag_name, assets })
}

/// Download a file from URL and return its contents
fn download_file(url: &str) -> Result<Vec<u8>, String> {
    let response = ureq::get(url)
        .set("User-Agent", "pte-self-updater")
        .call()
        .map_err(|e| format!("Failed to download: {}", e))?;

    let mut bytes = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Failed to read download: {}", e))?;

    Ok(bytes)
}

/// Compute SHA256 hash of data
fn compute_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    hex::encode(result)
}

/// Simple hex encoding (to avoid adding another dependency)
mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes
            .as_ref()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }
}

/// Extract expected checksum for a binary from checksums.txt content
fn extract_checksum(checksums_content: &str, binary_name: &str) -> Option<String> {
    for line in checksums_content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 && parts[1] == binary_name {
            return Some(parts[0].to_lowercase());
        }
    }
    None
}

/// Perform the self-update
pub fn do_update() -> Result<(), String> {
    println!("Checking for updates...");

    // Detect platform
    let asset_name = get_platform_asset_name()?;
    println!("Platform: {}", asset_name);

    // Fetch latest release
    let release = fetch_latest_release()?;
    let latest_version = &release.tag_name;

    println!("Current version: v{}", CURRENT_VERSION);
    println!("Latest version:  {}", latest_version);

    // Compare versions
    if !is_newer_version(CURRENT_VERSION, latest_version) {
        println!("Already up to date!");
        return Ok(());
    }

    println!("New version available! Downloading...");

    // Find the binary asset
    let binary_asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| format!("Binary not found for platform: {}", asset_name))?;

    // Find checksums.txt asset
    let checksums_asset = release
        .assets
        .iter()
        .find(|a| a.name == "checksums.txt")
        .ok_or("checksums.txt not found in release")?;

    // Download checksums first
    print!("Downloading checksums... ");
    io::stdout().flush().ok();
    let checksums_content = download_file(&checksums_asset.browser_download_url)?;
    let checksums_str =
        String::from_utf8(checksums_content).map_err(|_| "Invalid checksums.txt encoding")?;
    println!("done");

    // Extract expected checksum
    let expected_checksum = extract_checksum(&checksums_str, &asset_name)
        .ok_or_else(|| format!("Checksum not found for {}", asset_name))?;

    // Download binary
    print!("Downloading {}... ", asset_name);
    io::stdout().flush().ok();
    let binary_data = download_file(&binary_asset.browser_download_url)?;
    println!("done ({} bytes)", binary_data.len());

    // Verify checksum
    print!("Verifying checksum... ");
    io::stdout().flush().ok();
    let actual_checksum = compute_sha256(&binary_data);
    if actual_checksum != expected_checksum {
        return Err(format!(
            "Checksum mismatch!\nExpected: {}\nActual:   {}",
            expected_checksum, actual_checksum
        ));
    }
    println!("OK");

    // Get current executable path
    let current_exe = env::current_exe().map_err(|e| format!("Failed to get current exe: {}", e))?;
    println!("Replacing: {}", current_exe.display());

    // Write to temp file first
    let temp_path = current_exe.with_extension("new");
    fs::write(&temp_path, &binary_data)
        .map_err(|e| format!("Failed to write temp file: {}", e))?;

    // Set executable permission on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&temp_path)
            .map_err(|e| format!("Failed to get temp file metadata: {}", e))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&temp_path, perms)
            .map_err(|e| format!("Failed to set executable permission: {}", e))?;
    }

    // Replace the current executable
    fs::rename(&temp_path, &current_exe)
        .map_err(|e| format!("Failed to replace executable: {}", e))?;

    println!("Successfully updated to {}!", latest_version);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version() {
        assert_eq!(parse_version("1.0.0"), Some((1, 0, 0)));
        assert_eq!(parse_version("v1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version("0.10.5"), Some((0, 10, 5)));
    }

    #[test]
    fn test_is_newer_version() {
        assert!(is_newer_version("1.0.0", "1.0.1"));
        assert!(is_newer_version("1.0.0", "1.1.0"));
        assert!(is_newer_version("1.0.0", "2.0.0"));
        assert!(!is_newer_version("1.0.0", "1.0.0"));
        assert!(!is_newer_version("1.0.1", "1.0.0"));
        assert!(is_newer_version("1.0.0", "v1.0.1"));
    }

    #[test]
    fn test_extract_checksum() {
        let checksums = "abc123def456  pte-linux-x86_64\n789xyz  pte-macos-aarch64\n";
        assert_eq!(
            extract_checksum(checksums, "pte-linux-x86_64"),
            Some("abc123def456".to_string())
        );
        assert_eq!(
            extract_checksum(checksums, "pte-macos-aarch64"),
            Some("789xyz".to_string())
        );
        assert_eq!(extract_checksum(checksums, "pte-windows"), None);
    }

    #[test]
    fn test_get_platform_asset_name() {
        // This test will pass on supported platforms
        let result = get_platform_asset_name();
        // On Linux x86_64, it should return Ok
        if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            assert_eq!(result, Ok("pte-linux-x86_64".to_string()));
        }
        if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            assert_eq!(result, Ok("pte-macos-aarch64".to_string()));
        }
    }
}
