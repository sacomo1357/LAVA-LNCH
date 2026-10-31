//! Self-update from GitHub Releases: compare the latest release tag with this build's
//! version, download the Windows installer, run it, and relaunch.

use crate::{settings, util::*};
use serde::Serialize;
use serde_json::Value;
use tauri::AppHandle;

/// Build-time default release source. Fork the launcher? Change it in launcher.config.json.
const CONFIG: &str = include_str!("../../launcher.config.json");

pub fn default_repo() -> String {
    serde_json::from_str::<Value>(CONFIG)
        .ok()
        .and_then(|v| v["update_repo"].as_str().map(String::from))
        .unwrap_or_default()
}

/// "owner/repo" from either that form or any github.com URL of the repo.
pub fn parse_repo(s: &str) -> Option<(String, String)> {
    let s = s.trim().trim_end_matches('/');
    let s = s
        .strip_prefix("https://github.com/")
        .or_else(|| s.strip_prefix("http://github.com/"))
        .or_else(|| s.strip_prefix("github.com/"))
        .unwrap_or(s);
    let mut it = s.split('/');
    let owner = it.next()?.trim();
    let repo = it.next()?.trim().trim_end_matches(".git");
    let ok = |x: &str| !x.is_empty() && x.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c));
    (ok(owner) && ok(repo)).then(|| (owner.to_string(), repo.to_string()))
}

/// Numeric dotted versions; "v" prefix and pre-release suffixes are ignored.
pub fn newer(latest: &str, current: &str) -> bool {
    let nums = |v: &str| -> Vec<u64> {
        v.trim()
            .trim_start_matches(['v', 'V'])
            .split(['-', '+'])
            .next()
            .unwrap_or("")
            .split('.')
            .map(|p| p.parse().unwrap_or(0))
            .collect()
    };
    let (a, b) = (nums(latest), nums(current));
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (a.get(i).copied().unwrap_or(0), b.get(i).copied().unwrap_or(0));
        if x != y {
            return x > y;
        }
    }
    false
}

#[derive(Serialize, Clone)]
pub struct UpdateInfo {
    current: String,
    latest: String,
    available: bool,
    /// false in development builds, where replacing the running exe makes no sense
    can_install: bool,
    repo: String,
    notes: String,
    page: String,
    asset_name: String,
    asset_url: String,
    size: u64,
}

fn repo_for(app: &AppHandle) -> R<(String, String)> {
    let s = settings::load(app);
    let raw = if s.update_repo.trim().is_empty() { default_repo() } else { s.update_repo.clone() };
    parse_repo(&raw).ok_or_else(|| tf("err.repoInvalid", &[("repo", &raw)]))
}

#[tauri::command]
pub async fn update_check(app: AppHandle) -> R<UpdateInfo> {
    let (owner, repo) = repo_for(&app)?;
    let current = app.package_info().version.to_string();
    let r = http()
        .get(format!("https://api.github.com/repos/{owner}/{repo}/releases/latest"))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(err)?;
    if r.status().as_u16() == 404 {
        return Err(tf("err.noRelease", &[("repo", &format!("{owner}/{repo}"))]));
    }
    let v: Value = r.error_for_status().map_err(err)?.json().await.map_err(err)?;
    let latest = v["tag_name"].as_str().unwrap_or("").trim_start_matches(['v', 'V']).to_string();
    let assets = v["assets"].as_array().cloned().unwrap_or_default();
    // NSIS setup first (installs per-user, no admin prompt), MSI as a fallback
    let asset = assets
        .iter()
        .find(|a| a["name"].as_str().is_some_and(|n| n.ends_with("-setup.exe")))
        .or_else(|| assets.iter().find(|a| a["name"].as_str().is_some_and(|n| n.ends_with(".msi"))));
    Ok(UpdateInfo {
        available: !latest.is_empty() && newer(&latest, &current) && asset.is_some(),
        can_install: !cfg!(debug_assertions),
        current,
        latest,
        repo: format!("{owner}/{repo}"),
        notes: v["body"].as_str().unwrap_or("").to_string(),
        page: v["html_url"].as_str().unwrap_or("").to_string(),
        asset_name: asset.and_then(|a| a["name"].as_str()).unwrap_or("").to_string(),
        asset_url: asset.and_then(|a| a["browser_download_url"].as_str()).unwrap_or("").to_string(),
        size: asset.and_then(|a| a["size"].as_u64()).unwrap_or(0),
    })
}

/// PowerShell single-quoted string literal.
fn ps_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

#[tauri::command]
pub async fn update_install(app: AppHandle) -> R<()> {
    let info = update_check(app.clone()).await?;
    if !info.available {
        return Err(t("err.upToDate"));
    }
    if !info.can_install {
        return Err(t("err.devBuild"));
    }
    if !safe_name(&info.asset_name) || !info.asset_url.starts_with("https://") {
        return Err(t("err.badAsset"));
    }
    let dest = root(&app).join("updates").join(&info.asset_name);
    let _ = std::fs::remove_file(&dest);
    let prog = Prog::new(&app);
    prog.stage(&tf("stage.updateDownload", &[("version", &info.latest)]));
    let r = download_all(vec![Item { url: info.asset_url.clone(), dest: dest.clone(), sha1: None }], &prog, 1).await;
    prog.finish();
    r?;
    let got = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
    if info.size > 0 && got != info.size {
        let _ = std::fs::remove_file(&dest);
        return Err(tf("err.incomplete", &[("got", &got), ("size", &info.size)]));
    }

    // Wait for this launcher to exit, install quietly, then start the new version.
    let exe = std::env::current_exe().map_err(err)?;
    let install = if info.asset_name.ends_with(".msi") {
        format!("Start-Process -Wait msiexec -ArgumentList '/i',{},'/passive'", ps_quote(&dest.to_string_lossy()))
    } else {
        format!("Start-Process -Wait -FilePath {} -ArgumentList '/P'", ps_quote(&dest.to_string_lossy()))
    };
    let script = format!(
        "Wait-Process -Id {} -ErrorAction SilentlyContinue; {install}; Start-Process -FilePath {}",
        std::process::id(),
        ps_quote(&exe.to_string_lossy())
    );
    let mut cmd = std::process::Command::new("powershell");
    cmd.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-WindowStyle", "Hidden", "-Command", &script]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    cmd.spawn().map_err(|e| tf("err.installerFail", &[("detail", &e)]))?;
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub fn update_default_repo() -> String {
    default_repo()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        assert!(newer("v0.2.0", "0.1.9"));
        assert!(newer("1.0.0", "0.9.9"));
        assert!(newer("0.1.10", "0.1.9"));
        assert!(newer("0.1.0.1", "0.1.0"));
        assert!(!newer("0.1.0", "0.1.0"));
        assert!(!newer("v0.1.0-beta", "0.1.0"));
        assert!(!newer("0.0.9", "0.1.0"));
    }

    #[test]
    fn repos() {
        let want = Some(("sacomo1357".to_string(), "LAVA-LNCH".to_string()));
        assert_eq!(parse_repo("sacomo1357/LAVA-LNCH"), want);
        assert_eq!(parse_repo("https://github.com/sacomo1357/LAVA-LNCH.git"), want);
        assert_eq!(parse_repo("https://github.com/sacomo1357/LAVA-LNCH/releases/latest"), want);
        assert_eq!(parse_repo("github.com/sacomo1357/LAVA-LNCH/"), want);
        assert_eq!(parse_repo("evil;rm/x"), None);
        assert_eq!(parse_repo("onlyowner"), None);
        assert_eq!(default_repo(), "sacomo1357/LAVA-LNCH");
    }
}
