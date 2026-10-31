use crate::util::*;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Account {
    pub kind: String, // "offline" | "microsoft"
    pub name: String,
    pub uuid: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: u64,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Settings {
    pub java_path: String,
    pub ram_min: u32,
    pub ram_max: u32,
    pub jvm_preset: String, // none | aikar | zgc | shenandoah
    pub extra_jvm: String,
    pub launcher_behavior: String, // hide | close | keep
    pub ms_client_id: String,
    pub last_server: String,
    pub account: Option<Account>,
    /// GitHub repo to update from ("owner/repo" or URL); empty = launcher.config.json default
    pub update_repo: String,
    pub auto_update: bool,
}

/// Application (client) ID of the "LAVA LNCH" Azure app. A client ID is public, so it ships in the binary.
/// A non-empty `ms_client_id` in settings overrides it.
pub const DEFAULT_MS_CLIENT_ID: &str = "08637dab-695c-4575-b7f6-a5156297273e";

impl Settings {
    pub fn client_id(&self) -> String {
        let own = self.ms_client_id.trim();
        if own.is_empty() { DEFAULT_MS_CLIENT_ID.to_string() } else { own.to_string() }
    }
}

#[tauri::command]
pub fn builtin_client_id() -> bool {
    !DEFAULT_MS_CLIENT_ID.is_empty()
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            java_path: String::new(),
            ram_min: 1024,
            ram_max: 4096,
            jvm_preset: "aikar".into(),
            extra_jvm: String::new(),
            launcher_behavior: "hide".into(),
            ms_client_id: String::new(),
            last_server: String::new(),
            account: None,
            update_repo: String::new(),
            auto_update: true,
        }
    }
}

pub fn load(app: &AppHandle) -> Settings {
    std::fs::read(root(app).join("settings.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn save(app: &AppHandle, s: &Settings) -> R<()> {
    let dir = root(app);
    std::fs::create_dir_all(&dir).map_err(err)?;
    std::fs::write(dir.join("settings.json"), serde_json::to_vec_pretty(s).map_err(err)?).map_err(err)
}

pub fn jvm_preset_flags(preset: &str) -> Vec<String> {
    let s = match preset {
        "aikar" => "-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200 -XX:+UnlockExperimentalVMOptions -XX:+DisableExplicitGC -XX:+AlwaysPreTouch -XX:G1NewSizePercent=30 -XX:G1MaxNewSizePercent=40 -XX:G1HeapRegionSize=8M -XX:G1ReservePercent=20 -XX:G1HeapWastePercent=5 -XX:G1MixedGCCountTarget=4 -XX:InitiatingHeapOccupancyPercent=15 -XX:G1MixedGCLiveThresholdPercent=90 -XX:G1RSetUpdatingPauseTimePercent=5 -XX:SurvivorRatio=32 -XX:+PerfDisableSharedMem -XX:MaxTenuringThreshold=1",
        "zgc" => "-XX:+UseZGC",
        "shenandoah" => "-XX:+UseShenandoahGC",
        _ => "",
    };
    s.split_whitespace().map(String::from).collect()
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Settings {
    load(&app)
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: Settings) -> R<Settings> {
    let mut s = settings;
    // The account is owned by the auth commands; never let a stale UI copy overwrite it.
    s.account = load(&app).account;
    save(&app, &s)?;
    Ok(s)
}

/// The user's custom background as a data URL, if one is set.
#[tauri::command]
pub fn bg_get(app: AppHandle) -> Option<String> {
    use base64::{engine::general_purpose::STANDARD as B64, Engine};
    let d = root(&app);
    let mime = std::fs::read_to_string(d.join("background.mime")).ok()?;
    let bytes = std::fs::read(d.join("background.bin")).ok()?;
    Some(format!("data:{};base64,{}", mime.trim(), B64.encode(bytes)))
}

/// Set (data URL) or clear (null) the custom background.
#[tauri::command]
pub fn bg_set(app: AppHandle, data_url: Option<String>) -> R<()> {
    use base64::{engine::general_purpose::STANDARD as B64, Engine};
    let d = root(&app);
    std::fs::create_dir_all(&d).map_err(err)?;
    let Some(u) = data_url else {
        let _ = std::fs::remove_file(d.join("background.bin"));
        let _ = std::fs::remove_file(d.join("background.mime"));
        return Ok(());
    };
    let (head, b64) = u.split_once(',').ok_or("bad image")?;
    let mime = head.strip_prefix("data:").and_then(|h| h.strip_suffix(";base64")).ok_or("bad image")?;
    if !["image/png", "image/jpeg", "image/webp", "image/gif"].contains(&mime) {
        return Err(t("err.bgType"));
    }
    let bytes = B64.decode(b64).map_err(err)?;
    if bytes.len() > 15 * 1024 * 1024 {
        return Err(t("err.bgSize"));
    }
    std::fs::write(d.join("background.bin"), bytes).map_err(err)?;
    std::fs::write(d.join("background.mime"), mime).map_err(err)
}

#[tauri::command]
pub fn open_url(url: String) -> R<()> {
    // opens the default browser; only plain https links are allowed
    let ok = url.starts_with("https://") && url.chars().all(|c| c.is_ascii_alphanumeric() || "/:._-?=&%#+~".contains(c));
    if !ok {
        return Err("refusing to open that link".into());
    }
    std::process::Command::new("rundll32").args(["url.dll,FileProtocolHandler", &url]).spawn().map_err(err)?;
    Ok(())
}

#[tauri::command]
pub fn open_folder(app: AppHandle, sub: String) -> R<()> {
    let mut p = root(&app);
    if !sub.is_empty() {
        if !safe_rel(&sub) {
            return Err("bad path".into());
        }
        p = p.join(sub);
    }
    std::fs::create_dir_all(&p).map_err(err)?;
    std::process::Command::new("explorer").arg(p).spawn().map_err(err)?;
    Ok(())
}
