use crate::util::*;
use futures_util::future::join_all;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tauri::AppHandle;

#[derive(Serialize, Clone)]
pub struct JavaInfo {
    pub path: String,
    pub major: u32,
    pub version: String,
}

fn parse(out: &str) -> Option<(u32, String)> {
    let i = out.find("version \"")?;
    let rest = &out[i + 9..];
    let v = &rest[..rest.find('"')?];
    let mut parts = v.split(['.', '_', '-', '+']);
    let first: u32 = parts.next()?.parse().ok()?;
    let major = if first == 1 { parts.next()?.parse().ok()? } else { first };
    Some((major, v.to_string()))
}

pub async fn probe(p: &Path) -> Option<JavaInfo> {
    let mut c = tokio::process::Command::new(p);
    c.arg("-version");
    #[cfg(windows)]
    c.creation_flags(0x08000000);
    let o = tokio::time::timeout(Duration::from_secs(8), c.output()).await.ok()?.ok()?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&o.stderr),
        String::from_utf8_lossy(&o.stdout)
    );
    let (major, version) = parse(&text)?;
    Some(JavaInfo { path: p.to_string_lossy().into(), major, version })
}

fn scan(dir: &Path, depth: u32, out: &mut Vec<PathBuf>) {
    if depth == 0 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if !p.is_dir() {
            continue;
        }
        let j = p.join("bin").join("java.exe");
        if j.is_file() {
            out.push(j);
        } else {
            scan(&p, depth - 1, out);
        }
    }
}

fn candidates(app: &AppHandle) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = vec![];
    if let Ok(h) = std::env::var("JAVA_HOME") {
        v.push(PathBuf::from(h).join("bin").join("java.exe"));
    }
    if let Some(p) = std::env::var_os("PATH") {
        for d in std::env::split_paths(&p) {
            v.push(d.join("java.exe"));
        }
    }
    for var in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Ok(pf) = std::env::var(var) {
            for n in ["Java", "Eclipse Adoptium", "Microsoft", "Zulu", "Amazon Corretto", "BellSoft", "Semeru"] {
                scan(&PathBuf::from(&pf).join(n), 2, &mut v);
            }
        }
    }
    scan(&root(app).join("runtime"), 3, &mut v);
    v.retain(|p| p.is_file());
    let mut seen = std::collections::HashSet::new();
    v.retain(|p| seen.insert(p.to_string_lossy().to_lowercase()));
    v
}

pub async fn detect(app: &AppHandle) -> Vec<JavaInfo> {
    let found = join_all(candidates(app).iter().map(|p| probe(p))).await;
    let mut out: Vec<JavaInfo> = found.into_iter().flatten().collect();
    out.sort_by_key(|j| j.major);
    out
}

fn extract(zip_path: &Path, dest: &Path) -> R<()> {
    let f = std::fs::File::open(zip_path).map_err(err)?;
    let mut z = zip::ZipArchive::new(f).map_err(err)?;
    for i in 0..z.len() {
        let mut e = z.by_index(i).map_err(err)?;
        let Some(rel) = e.enclosed_name() else { continue };
        let out = dest.join(rel);
        if e.is_dir() {
            std::fs::create_dir_all(&out).map_err(err)?;
        } else {
            if let Some(p) = out.parent() {
                std::fs::create_dir_all(p).map_err(err)?;
            }
            let mut o = std::fs::File::create(&out).map_err(err)?;
            std::io::copy(&mut e, &mut o).map_err(err)?;
        }
    }
    Ok(())
}

pub async fn install_temurin(app: &AppHandle, major: u32, prog: &Prog) -> R<PathBuf> {
    let dir = root(app).join("runtime").join(major.to_string());
    prog.stage(&format!("Downloading Java {major} (Temurin)"));
    let url = format!(
        "https://api.adoptium.net/v3/binary/latest/{major}/ga/windows/x64/jre/hotspot/normal/eclipse"
    );
    let zip = dir.join("jre.zip");
    let _ = tokio::fs::remove_file(&zip).await;
    download(&Item { url, dest: zip.clone(), sha1: None }).await?;
    prog.stage("Extracting Java");
    let (z, d) = (zip.clone(), dir.clone());
    tokio::task::spawn_blocking(move || extract(&z, &d)).await.map_err(err)??;
    let _ = tokio::fs::remove_file(&zip).await;
    let mut v = vec![];
    scan(&dir, 3, &mut v);
    v.into_iter().next().ok_or_else(|| "Java was not found after install".to_string())
}

/// Prefer the configured Java, then the smallest installed Java that is new enough,
/// then download Temurin.
pub async fn ensure(app: &AppHandle, configured: &str, major: u32, prog: &Prog) -> R<PathBuf> {
    if !configured.trim().is_empty() {
        if let Some(j) = probe(Path::new(configured.trim())).await {
            if j.major >= major {
                return Ok(PathBuf::from(j.path));
            }
        }
    }
    prog.stage("Looking for Java");
    let mut found = detect(app).await;
    found.retain(|j| j.major >= major);
    if let Some(j) = found.first() {
        return Ok(PathBuf::from(&j.path));
    }
    install_temurin(app, major, prog).await
}

#[tauri::command]
pub async fn java_detect(app: AppHandle) -> Vec<JavaInfo> {
    detect(&app).await
}

#[tauri::command]
pub async fn java_install(app: AppHandle, major: u32) -> R<String> {
    let prog = Prog::new(&app);
    let r = install_temurin(&app, major, &prog).await;
    prog.finish();
    r.map(|p| p.to_string_lossy().to_string())
}
