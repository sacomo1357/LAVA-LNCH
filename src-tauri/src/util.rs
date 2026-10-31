use futures_util::{stream, StreamExt};
use serde::Serialize;
use serde_json::Value;
use sha1::{Digest, Sha1};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, OnceLock,
    },
};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::AsyncWriteExt;

pub use crate::i18n::{t, tf};

pub type R<T> = Result<T, String>;

pub fn err<E: std::fmt::Display>(x: E) -> String {
    x.to_string()
}

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn http() -> &'static reqwest::Client {
    static C: OnceLock<reqwest::Client> = OnceLock::new();
    C.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent("LavaSE-Launcher/0.1 (lava-se)")
            .connect_timeout(std::time::Duration::from_secs(15))
            .build()
            .expect("http client")
    })
}

pub fn root(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("lava")
}

pub async fn get_json(url: &str) -> R<Value> {
    get_json_q(url, &[]).await
}

pub async fn get_json_q(url: &str, q: &[(&str, &str)]) -> R<Value> {
    http()
        .get(url)
        .query(q)
        .send()
        .await
        .map_err(err)?
        .error_for_status()
        .map_err(err)?
        .json()
        .await
        .map_err(err)
}

pub async fn sha1_of(p: &Path) -> Option<String> {
    let b = tokio::fs::read(p).await.ok()?;
    Some(hex::encode(Sha1::digest(&b)))
}

#[derive(Clone)]
pub struct Item {
    pub url: String,
    pub dest: PathBuf,
    pub sha1: Option<String>,
}

pub async fn download(item: &Item) -> R<()> {
    if item.dest.exists() {
        match &item.sha1 {
            Some(h) => {
                if sha1_of(&item.dest).await.as_deref() == Some(h.as_str()) {
                    return Ok(());
                }
            }
            None => return Ok(()),
        }
    }
    if let Some(p) = item.dest.parent() {
        tokio::fs::create_dir_all(p).await.map_err(err)?;
    }
    let mut last = String::new();
    for _ in 0..3 {
        match try_download(item).await {
            Ok(()) => return Ok(()),
            Err(e) => last = e,
        }
    }
    Err(format!("{}: {}", item.url, last))
}

async fn try_download(item: &Item) -> R<()> {
    let resp = http()
        .get(&item.url)
        .send()
        .await
        .map_err(err)?
        .error_for_status()
        .map_err(err)?;
    let mut tmp = item.dest.as_os_str().to_owned();
    tmp.push(".part");
    let tmp = PathBuf::from(tmp);
    let mut f = tokio::fs::File::create(&tmp).await.map_err(err)?;
    let mut h = Sha1::new();
    let mut st = resp.bytes_stream();
    while let Some(c) = st.next().await {
        let c = c.map_err(err)?;
        h.update(&c);
        f.write_all(&c).await.map_err(err)?;
    }
    f.flush().await.map_err(err)?;
    drop(f);
    if let Some(exp) = &item.sha1 {
        let got = hex::encode(h.finalize());
        if !got.eq_ignore_ascii_case(exp) {
            let _ = tokio::fs::remove_file(&tmp).await;
            return Err(format!("sha1 mismatch (expected {exp}, got {got})"));
        }
    }
    tokio::fs::rename(&tmp, &item.dest).await.map_err(err)
}

#[derive(Clone, Serialize)]
struct ProgressEvt {
    label: String,
    done: u64,
    total: u64,
    active: bool,
}

#[derive(Clone)]
pub struct Prog {
    app: AppHandle,
    done: Arc<AtomicU64>,
    total: Arc<AtomicU64>,
}

impl Prog {
    pub fn new(app: &AppHandle) -> Self {
        Prog {
            app: app.clone(),
            done: Arc::new(AtomicU64::new(0)),
            total: Arc::new(AtomicU64::new(0)),
        }
    }
    fn emit(&self, label: &str, active: bool) {
        let _ = self.app.emit(
            "progress",
            ProgressEvt {
                label: label.to_string(),
                done: self.done.load(Ordering::Relaxed),
                total: self.total.load(Ordering::Relaxed),
                active,
            },
        );
    }
    pub fn stage(&self, label: &str) {
        self.emit(label, true);
    }
    pub fn add_total(&self, n: u64) {
        self.total.fetch_add(n, Ordering::Relaxed);
    }
    pub fn tick(&self, label: &str) {
        self.done.fetch_add(1, Ordering::Relaxed);
        self.emit(label, true);
    }
    pub fn finish(&self) {
        self.emit("", false);
    }
}

pub async fn download_all(items: Vec<Item>, prog: &Prog, conc: usize) -> R<()> {
    prog.add_total(items.len() as u64);
    let results: Vec<R<()>> = stream::iter(items)
        .map(|it| {
            let p = prog.clone();
            async move {
                let r = download(&it).await;
                let name = it
                    .dest
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                p.tick(&name);
                r
            }
        })
        .buffer_unordered(conc)
        .collect()
        .await;
    for r in results {
        r?;
    }
    Ok(())
}

pub fn sanitize(s: &str) -> String {
    let t: String = s
        .trim()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
        .take(64)
        .collect();
    t.trim_matches('.').to_string()
}

pub fn safe_name(s: &str) -> bool {
    !s.is_empty() && !s.contains(['/', '\\', ':']) && s != "." && s != ".."
}

/// A relative path from an untrusted manifest: no traversal, no drive letters.
pub fn safe_rel(p: &str) -> bool {
    !p.is_empty()
        && !p.starts_with('/')
        && !p.contains('\\')
        && !p.contains(':')
        && p.split('/').all(|c| !c.is_empty() && c != "." && c != "..")
}
