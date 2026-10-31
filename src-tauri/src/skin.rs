use crate::{auth, settings, util::*};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::AppHandle;

#[derive(Serialize)]
pub struct SkinData {
    png_b64: Option<String>,
    variant: String, // "classic" | "slim"
    source: String,  // "microsoft" | "local" | "lookup" | "none"
}

async fn fetch_png(url: &str) -> R<Vec<u8>> {
    let url = url.replacen("http://", "https://", 1);
    Ok(http().get(url).send().await.map_err(err)?.error_for_status().map_err(err)?.bytes().await.map_err(err)?.to_vec())
}

/// Look up a public skin by player name (used for offline accounts whose name matches a real player).
async fn lookup(name: &str) -> Option<(Vec<u8>, String)> {
    let id = get_json(&format!("https://api.mojang.com/users/profiles/minecraft/{name}")).await.ok()?;
    let prof = get_json(&format!("https://sessionserver.mojang.com/session/minecraft/profile/{}", id["id"].as_str()?)).await.ok()?;
    let raw = prof["properties"].as_array()?.iter().find(|p| p["name"] == "textures")?["value"].as_str()?;
    let tex: Value = serde_json::from_slice(&B64.decode(raw).ok()?).ok()?;
    let skin = &tex["textures"]["SKIN"];
    let variant = if skin["metadata"]["model"] == "slim" { "slim" } else { "classic" };
    Some((fetch_png(skin["url"].as_str()?).await.ok()?, variant.into()))
}

fn local_paths(app: &AppHandle, name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let d = root(app).join("skins");
    (d.join(format!("{}.png", sanitize(name))), d.join(format!("{}.variant", sanitize(name))))
}

/// The skin currently applied to the logged-in account.
#[tauri::command]
pub async fn skin_get(app: AppHandle) -> R<SkinData> {
    let acc = settings::load(&app).account.ok_or("Log in first")?;
    if acc.kind == "microsoft" {
        let acc = auth::fresh_account(&app).await?;
        let prof: Value = http()
            .get("https://api.minecraftservices.com/minecraft/profile")
            .bearer_auth(&acc.access_token)
            .send()
            .await
            .map_err(err)?
            .error_for_status()
            .map_err(err)?
            .json()
            .await
            .map_err(err)?;
        let skin = prof["skins"].as_array().and_then(|a| a.iter().find(|s| s["state"] == "ACTIVE").or_else(|| a.first()));
        if let Some(s) = skin {
            let png = fetch_png(s["url"].as_str().unwrap_or_default()).await?;
            let variant = if s["variant"] == "SLIM" { "slim" } else { "classic" };
            return Ok(SkinData { png_b64: Some(B64.encode(png)), variant: variant.into(), source: "microsoft".into() });
        }
    } else {
        let (png, var) = local_paths(&app, &acc.name);
        if let Ok(b) = std::fs::read(&png) {
            let v = std::fs::read_to_string(var).unwrap_or_default();
            return Ok(SkinData { png_b64: Some(B64.encode(b)), variant: if v.trim() == "slim" { "slim" } else { "classic" }.into(), source: "local".into() });
        }
        if let Some((b, v)) = lookup(&acc.name).await {
            return Ok(SkinData { png_b64: Some(B64.encode(b)), variant: v, source: "lookup".into() });
        }
    }
    Ok(SkinData { png_b64: None, variant: "classic".into(), source: "none".into() })
}

/// A Minecraft skin: PNG, 64 wide, 64 or 32 tall.
fn check_skin(bytes: &[u8]) -> R<()> {
    let ok_png = bytes.len() < 1024 * 1024 && bytes.len() > 24 && bytes.starts_with(&[0x89, b'P', b'N', b'G']);
    if !ok_png {
        return Err("스킨은 PNG 파일이어야 합니다".into());
    }
    let w = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let h = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    if w != 64 || (h != 64 && h != 32) {
        return Err(format!("스킨 크기는 64x64 (또는 64x32)여야 합니다. 이 파일은 {w}x{h}입니다"));
    }
    Ok(())
}

async fn apply(app: &AppHandle, bytes: Vec<u8>, variant: &str) -> R<String> {
    check_skin(&bytes)?;
    let variant = if variant == "slim" { "slim" } else { "classic" };
    let acc = settings::load(app).account.ok_or("Log in first")?;
    if acc.kind != "microsoft" {
        let (png, var) = local_paths(app, &acc.name);
        std::fs::create_dir_all(png.parent().unwrap()).map_err(err)?;
        std::fs::write(png, bytes).map_err(err)?;
        std::fs::write(var, variant).map_err(err)?;
        return Ok("로컬에 적용했습니다. 오프라인 계정은 게임 안 스킨이 바뀌지 않습니다.".into());
    }
    let acc = auth::fresh_account(app).await?;
    let form = reqwest::multipart::Form::new().text("variant", variant).part(
        "file",
        reqwest::multipart::Part::bytes(bytes).file_name("skin.png").mime_str("image/png").map_err(err)?,
    );
    let r = http()
        .post("https://api.minecraftservices.com/minecraft/profile/skins")
        .bearer_auth(&acc.access_token)
        .multipart(form)
        .send()
        .await
        .map_err(err)?;
    if !r.status().is_success() {
        return Err(format!("Mojang이 스킨을 거부했습니다 ({})", r.status()));
    }
    Ok("마인크래프트 계정에 스킨을 적용했습니다.".into())
}

// ---- skin library ----

#[derive(Serialize, Deserialize, Clone)]
pub struct SkinEntry {
    id: String,
    name: String,
    variant: String,
    added: u64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    png_b64: String,
}

fn lib_dir(app: &AppHandle) -> std::path::PathBuf {
    root(app).join("skins").join("library")
}

fn lib_index(app: &AppHandle) -> Vec<SkinEntry> {
    std::fs::read(lib_dir(app).join("index.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn lib_save(app: &AppHandle, v: &[SkinEntry]) -> R<()> {
    let d = lib_dir(app);
    std::fs::create_dir_all(&d).map_err(err)?;
    std::fs::write(d.join("index.json"), serde_json::to_vec_pretty(v).map_err(err)?).map_err(err)
}

fn lib_png(app: &AppHandle, id: &str) -> R<std::path::PathBuf> {
    if !id.chars().all(|c| c.is_ascii_hexdigit()) || id.is_empty() {
        return Err("bad skin id".into());
    }
    Ok(lib_dir(app).join(format!("{id}.png")))
}

#[tauri::command]
pub fn skin_list(app: AppHandle) -> Vec<SkinEntry> {
    lib_index(&app)
        .into_iter()
        .filter_map(|mut e| {
            e.png_b64 = B64.encode(std::fs::read(lib_png(&app, &e.id).ok()?).ok()?);
            Some(e)
        })
        .collect()
}

#[tauri::command]
pub fn skin_add(app: AppHandle, name: String, png_b64: String, variant: String) -> R<SkinEntry> {
    let bytes = B64.decode(png_b64).map_err(err)?;
    check_skin(&bytes)?;
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 32 {
        return Err("스킨 이름은 1-32자로 입력하세요".into());
    }
    let added = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    let e = SkinEntry {
        id: format!("{added:x}"),
        name: name.to_string(),
        variant: if variant == "slim" { "slim" } else { "classic" }.into(),
        added: (added / 1000) as u64,
        png_b64: String::new(),
    };
    std::fs::create_dir_all(lib_dir(&app)).map_err(err)?;
    std::fs::write(lib_png(&app, &e.id)?, bytes).map_err(err)?;
    let mut idx = lib_index(&app);
    idx.push(e.clone());
    lib_save(&app, &idx)?;
    Ok(e)
}

#[tauri::command]
pub fn skin_delete(app: AppHandle, id: String) -> R<()> {
    let _ = std::fs::remove_file(lib_png(&app, &id)?);
    let idx: Vec<SkinEntry> = lib_index(&app).into_iter().filter(|e| e.id != id).collect();
    lib_save(&app, &idx)
}

#[tauri::command]
pub async fn skin_apply(app: AppHandle, id: String) -> R<String> {
    let e = lib_index(&app).into_iter().find(|e| e.id == id).ok_or("스킨을 찾을 수 없습니다")?;
    let bytes = std::fs::read(lib_png(&app, &id)?).map_err(err)?;
    apply(&app, bytes, &e.variant).await
}
