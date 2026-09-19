use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use eframe::egui;

const RELEASE_API: &str = "https://api.github.com/repos/Mobil0010/resource_monitor/releases/latest";

#[derive(Clone, Debug, PartialEq)]
pub(super) struct UpdateInfo {
    pub(super) version: String,
    pub(super) asset_url: String,
    pub(super) asset_name: String,
}

pub(super) fn start_check(ctx: egui::Context) -> Receiver<Option<UpdateInfo>> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let update = fetch_latest().ok().flatten();
        let _ = sender.send(update);
        ctx.request_repaint();
    });
    receiver
}

fn fetch_latest() -> Result<Option<UpdateInfo>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(8)))
        .build()
        .into();
    let mut response = agent
        .get(RELEASE_API)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header(
            "User-Agent",
            concat!("ResourceMonitor/", env!("CARGO_PKG_VERSION")),
        )
        .call()
        .map_err(|error| error.to_string())?;
    let value: serde_json::Value = response
        .body_mut()
        .read_json()
        .map_err(|error| error.to_string())?;
    Ok(latest_from_json(&value))
}

pub(super) fn latest_from_json(value: &serde_json::Value) -> Option<UpdateInfo> {
    let tag = value.get("tag_name")?.as_str()?;
    let version_text = tag.strip_prefix('v').unwrap_or(tag);
    let latest = semver::Version::parse(version_text).ok()?;
    let current = semver::Version::parse(env!("CARGO_PKG_VERSION")).ok()?;
    if latest <= current {
        return None;
    }
    let url = value.get("html_url")?.as_str()?;
    if !url.starts_with("https://github.com/Mobil0010/resource_monitor/releases/tag/") {
        return None;
    }
    let suffix = if cfg!(target_os = "macos") {
        "-macOS-Universal.dmg"
    } else if cfg!(target_os = "windows") {
        "-Windows-Setup.exe"
    } else {
        return None;
    };
    let asset = value.get("assets")?.as_array()?.iter().find(|asset| {
        asset
            .get("name")
            .and_then(|name| name.as_str())
            .is_some_and(|name| name.starts_with("ResourceMonitor-") && name.ends_with(suffix))
    })?;
    let asset_name = asset.get("name")?.as_str()?;
    let asset_url = asset.get("browser_download_url")?.as_str()?;
    if !asset_url.starts_with("https://github.com/Mobil0010/resource_monitor/releases/download/")
        || asset_name.contains(['/', '\\'])
    {
        return None;
    }
    Some(UpdateInfo {
        version: format!("v{latest}"),
        asset_url: asset_url.to_owned(),
        asset_name: asset_name.to_owned(),
    })
}

pub(super) fn start_download(
    update: UpdateInfo,
    ctx: egui::Context,
) -> Receiver<Result<PathBuf, String>> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let result = download(&update);
        let _ = sender.send(result);
        ctx.request_repaint();
    });
    receiver
}

fn download(update: &UpdateInfo) -> Result<PathBuf, String> {
    let directory = std::env::temp_dir().join("ResourceMonitorUpdate");
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let path = directory.join(&update.asset_name);
    let partial = directory.join(format!("{}.part", update.asset_name));
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(300)))
        .build()
        .into();
    let mut response = agent
        .get(&update.asset_url)
        .header(
            "User-Agent",
            concat!("ResourceMonitor/", env!("CARGO_PKG_VERSION")),
        )
        .call()
        .map_err(|error| error.to_string())?;
    let mut file = std::fs::File::create(&partial).map_err(|error| error.to_string())?;
    std::io::copy(&mut response.body_mut().as_reader(), &mut file)
        .map_err(|error| error.to_string())?;
    std::fs::rename(&partial, &path).map_err(|error| error.to_string())?;
    Ok(path)
}

pub(super) fn launch(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let result = Command::new("open").arg(path).spawn();
    #[cfg(target_os = "windows")]
    let result = {
        use std::os::windows::process::CommandExt;
        let installer = path.to_string_lossy().replace('\'', "''");
        let command = format!(
            "$installer='{installer}'; Wait-Process -Id {} -ErrorAction SilentlyContinue; Start-Process -FilePath $installer",
            std::process::id()
        );
        Command::new("C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-WindowStyle",
                "Hidden",
                "-Command",
            ])
            .arg(command)
            .creation_flags(0x08000000)
            .spawn()
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let result: std::io::Result<std::process::Child> = Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "unsupported platform",
    ));
    result.map(|_| ()).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::{UpdateInfo, latest_from_json};

    #[test]
    fn newer_release_is_detected() {
        let suffix = if cfg!(target_os = "macos") {
            "-macOS-Universal.dmg"
        } else {
            "-Windows-Setup.exe"
        };
        let asset_name = format!("ResourceMonitor-99.2.1{suffix}");
        let value = serde_json::json!({
            "tag_name": "v99.2.1",
            "html_url": "https://github.com/Mobil0010/resource_monitor/releases/tag/v99.2.1",
            "assets": [{
                "name": asset_name,
                "browser_download_url": format!("https://github.com/Mobil0010/resource_monitor/releases/download/v99.2.1/{asset_name}")
            }]
        });
        assert_eq!(
            latest_from_json(&value),
            Some(UpdateInfo {
                version: "v99.2.1".into(),
                asset_url: format!(
                    "https://github.com/Mobil0010/resource_monitor/releases/download/v99.2.1/{asset_name}"
                ),
                asset_name,
            })
        );
    }

    #[test]
    fn old_invalid_or_untrusted_releases_are_ignored() {
        for value in [
            serde_json::json!({
                "tag_name": env!("CARGO_PKG_VERSION"),
                "html_url": "https://github.com/Mobil0010/resource_monitor/releases/tag/current"
            }),
            serde_json::json!({
                "tag_name": "not-a-version",
                "html_url": "https://github.com/Mobil0010/resource_monitor/releases/tag/test"
            }),
            serde_json::json!({
                "tag_name": "v99.0.0",
                "html_url": "https://example.com/download"
            }),
        ] {
            assert_eq!(latest_from_json(&value), None);
        }
    }
}
