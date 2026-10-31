use crate::instances::{inst_dir, load_inst, save_inst, Instance};
use crate::mc::{self, LaunchCfg};
use crate::{settings, util::*};
use serde::Deserialize;
use std::{collections::HashSet, path::Path, time::Duration};
use tauri::AppHandle;

#[derive(Deserialize)]
struct LoaderSpec {
    r#type: String,
    version: String,
}

#[derive(Deserialize)]
struct FileSpec {
    path: String,
    sha1: String,
    url: String,
}

#[derive(Deserialize)]
struct Manifest {
    minecraft_version: String,
    loader: Option<LoaderSpec>,
    #[serde(default)]
    required_files: Vec<FileSpec>,
    // optional display info
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    icon: String,
    #[serde(default)]
    background: String,
}

/// Directories whose unlisted files are removed on sync (so old mods cannot conflict).
const MANAGED: [&str; 3] = ["mods", "resourcepacks", "shaderpacks"];

fn parse_addr(s: &str) -> R<(String, u16)> {
    let s = s.trim().trim_start_matches("https://").trim_start_matches("http://");
    let s = s.split('/').next().unwrap_or("");
    let (host, port) = match s.rsplit_once(':') {
        Some((h, p)) => (h, p.parse::<u16>().map_err(|_| "Invalid port".to_string())?),
        None => (s, 25565),
    };
    if host.is_empty() || !host.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-') {
        return Err("서버 도메인을 입력하세요 (예: play.example.com)".into());
    }
    Ok((host.to_string(), port))
}

/// Fetch the manifest and return it with the URL it came from (for resolving relative links).
async fn fetch_manifest(input: &str) -> R<(Manifest, String)> {
    let input = input.trim();
    let (host, _) = parse_addr(input)?;
    let mut urls: Vec<String> = vec![];
    if input.starts_with("http://") || input.starts_with("https://") {
        urls.push(if input.ends_with(".json") { input.to_string() } else { format!("{}/launcher/manifest.json", input.trim_end_matches('/')) });
    } else {
        for h in [host.clone(), format!("api.{host}")] {
            urls.push(format!("https://{h}/launcher/manifest.json"));
            urls.push(format!("http://{h}/launcher/manifest.json"));
        }
    }
    let mut last = String::from("no manifest URL");
    for u in urls {
        let r = http().get(&u).timeout(Duration::from_secs(8)).send().await;
        match r {
            Ok(resp) if resp.status().is_success() => {
                let m = resp.json::<Manifest>().await.map_err(|e| format!("Bad manifest at {u}: {e}"))?;
                return Ok((m, u));
            }
            Ok(resp) => last = format!("{u} -> {}", resp.status()),
            Err(e) => last = format!("{u} -> {e}"),
        }
    }
    Err(format!("서버 manifest를 불러오지 못했습니다 ({last})"))
}

/// Absolute http(s)/data URL for an icon or background, or "" if unusable.
fn resolve_link(base: &str, link: &str) -> String {
    let link = link.trim();
    if link.is_empty() {
        return String::new();
    }
    if link.starts_with("data:image/") {
        return link.to_string();
    }
    match reqwest::Url::parse(base).and_then(|b| b.join(link)) {
        Ok(u) if u.scheme() == "https" || u.scheme() == "http" => u.to_string(),
        _ => String::new(),
    }
}

fn inst_name(host: &str, port: u16) -> String {
    sanitize(&format!("srv-{host}{}", if port == 25565 { String::new() } else { format!("-{port}") }))
}

/// Server profile updated with the manifest's version and display info; keeps install state.
fn server_inst(app: &AppHandle, address: &str, m: &Manifest, murl: &str) -> R<Instance> {
    let (host, port) = parse_addr(address)?;
    let name = inst_name(&host, port);
    let mut i = load_inst(app, &name).unwrap_or_default();
    let lv = match &m.loader {
        Some(l) if l.r#type.eq_ignore_ascii_case("fabric") => Some(l.version.clone()),
        Some(l) if l.r#type.eq_ignore_ascii_case("vanilla") => None,
        Some(l) => return Err(format!("'{}' 로더는 아직 지원하지 않습니다 (Fabric만 지원)", l.r#type)),
        None => None,
    };
    if i.mc != m.minecraft_version || i.loader_version != lv.clone().unwrap_or_default() {
        i.version_id = String::new();
    }
    i.name = name;
    i.kind = "server".into();
    i.mc = m.minecraft_version.clone();
    i.loader = if lv.is_some() { "fabric".into() } else { "vanilla".into() };
    i.loader_version = lv.unwrap_or_default();
    i.server = format!("{host}:{port}");
    i.address = address.trim().to_string();
    i.title = if m.name.trim().is_empty() { host } else { m.name.trim().to_string() };
    i.description = m.description.trim().to_string();
    i.icon = resolve_link(murl, &m.icon);
    i.background = resolve_link(murl, &m.background);
    Ok(i)
}

/// Add (or refresh) a server in the list without downloading anything.
#[tauri::command]
pub async fn server_add(app: AppHandle, address: String) -> R<Instance> {
    let (m, murl) = fetch_manifest(&address).await?;
    let i = server_inst(&app, &address, &m, &murl)?;
    save_inst(&app, &i)?;
    Ok(i)
}

fn walk(dir: &Path, base: &Path, out: &mut Vec<(String, std::path::PathBuf)>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, base, out);
        } else if let Ok(rel) = p.strip_prefix(base) {
            out.push((rel.to_string_lossy().replace('\\', "/"), p));
        }
    }
}

#[tauri::command]
pub async fn multi_play(app: AppHandle, address: String) -> R<u32> {
    let prog = Prog::new(&app);
    let r = async {
        let (host, port) = parse_addr(&address)?;
        prog.stage("서버 manifest 확인 중");
        let (m, murl) = fetch_manifest(&address).await?;
        let mut inst = server_inst(&app, &address, &m, &murl)?;
        let dir = inst_dir(&app, &inst.name);

        let mut keep: HashSet<String> = HashSet::new();
        let mut items = vec![];
        for f in &m.required_files {
            if !safe_rel(&f.path) || f.path == "instance.json" {
                return Err(format!("Server manifest has an unsafe path: {}", f.path));
            }
            keep.insert(f.path.clone());
            items.push(Item { url: f.url.clone(), dest: dir.join(&f.path), sha1: Some(f.sha1.clone()) });
        }

        // remove files the server no longer lists, but only inside managed folders
        prog.stage("오래된 파일 정리 중");
        for d in MANAGED {
            let mut found = vec![];
            walk(&dir.join(d), &dir, &mut found);
            for (rel, p) in found {
                if !keep.contains(&rel) {
                    let _ = std::fs::remove_file(p);
                }
            }
        }

        download_all(items, &prog, 8).await?;
        let lv = (inst.loader == "fabric").then(|| inst.loader_version.clone());
        inst.version_id = mc::install(&app, &m.minecraft_version, lv.as_deref(), &prog).await?;
        save_inst(&app, &inst)?;
        let mut s = settings::load(&app);
        s.last_server = address.trim().to_string();
        settings::save(&app, &s)?;
        mc::launch(&app, LaunchCfg { version_id: inst.version_id, mc: inst.mc, dir, server: Some((host, port)) }, &prog).await
    }
    .await;
    prog.finish();
    r
}
