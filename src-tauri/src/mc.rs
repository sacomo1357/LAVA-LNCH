use crate::{auth, java, settings};
use crate::util::*;
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    process::Stdio,
};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};

const MANIFEST: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
/// Releases before 1.19 need native extraction, which this launcher does not do.
const MIN_RELEASE_TIME: &str = "2022-06-07";

pub fn versions_dir(app: &AppHandle) -> PathBuf {
    root(app).join("versions")
}

#[tauri::command]
pub async fn mc_versions() -> R<Value> {
    let m = get_json(MANIFEST).await?;
    let list: Vec<&str> = m["versions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|v| v["type"] == "release" && v["releaseTime"].as_str().unwrap_or("") >= MIN_RELEASE_TIME)
        .filter_map(|v| v["id"].as_str())
        .collect();
    Ok(json!({"latest": m["latest"]["release"], "versions": list}))
}

pub async fn latest_loader() -> R<String> {
    let v = get_json("https://meta.fabricmc.net/v2/versions/loader").await?;
    v.as_array()
        .and_then(|a| a.iter().find(|x| x["stable"] == true))
        .and_then(|x| x["version"].as_str())
        .map(String::from)
        .ok_or_else(|| "No Fabric loader found".to_string())
}

fn read_json(p: &Path) -> R<Value> {
    serde_json::from_slice(&std::fs::read(p).map_err(err)?).map_err(err)
}

fn write_json(p: &Path, v: &Value) -> R<()> {
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d).map_err(err)?;
    }
    std::fs::write(p, serde_json::to_vec(v).map_err(err)?).map_err(err)
}

/// Load a version JSON and merge it over its `inheritsFrom` parent (Fabric -> vanilla).
pub fn resolve(vd: &Path, id: &str) -> R<Value> {
    let v = read_json(&vd.join(id).join(format!("{id}.json")))?;
    let Some(parent) = v["inheritsFrom"].as_str() else { return Ok(v) };
    let mut p = resolve(vd, parent)?;
    let cat = |a: &Value, b: &Value| -> Value {
        let mut x = a.as_array().cloned().unwrap_or_default();
        x.extend(b.as_array().cloned().unwrap_or_default());
        Value::Array(x)
    };
    p["libraries"] = cat(&v["libraries"], &p["libraries"]);
    p["arguments"]["jvm"] = cat(&p["arguments"]["jvm"], &v["arguments"]["jvm"]);
    p["arguments"]["game"] = cat(&p["arguments"]["game"], &v["arguments"]["game"]);
    p["mainClass"] = v["mainClass"].clone();
    p["id"] = v["id"].clone();
    Ok(p)
}

fn rules_ok(rules: &Value, feats: &HashMap<&str, bool>) -> bool {
    let Some(arr) = rules.as_array() else { return true };
    let mut allowed = false;
    for r in arr {
        let mut m = true;
        if let Some(os) = r.get("os") {
            if os["name"].as_str().is_some_and(|n| n != "windows") {
                m = false;
            }
            if os["arch"].as_str().is_some_and(|a| a != "x86_64" && a != "amd64") {
                m = false;
            }
        }
        if let Some(f) = r.get("features").and_then(|f| f.as_object()) {
            for (k, v) in f {
                if feats.get(k.as_str()).copied().unwrap_or(false) != v.as_bool().unwrap_or(false) {
                    m = false;
                }
            }
        }
        if m {
            allowed = r["action"] == "allow";
        }
    }
    allowed
}

struct Lib {
    path: String,
    url: String,
    sha1: Option<String>,
}

fn libs(v: &Value) -> Vec<Lib> {
    let mut seen = HashSet::new();
    let mut out = vec![];
    for l in v["libraries"].as_array().into_iter().flatten() {
        if let Some(r) = l.get("rules") {
            if !rules_ok(r, &HashMap::new()) {
                continue;
            }
        }
        let name = l["name"].as_str().unwrap_or("");
        let p: Vec<&str> = name.split(':').collect();
        if p.len() < 3 {
            continue;
        }
        // first occurrence wins, so the loader's versions override vanilla's
        if !seen.insert(format!("{}:{}:{}", p[0], p[1], p.get(3).unwrap_or(&""))) {
            continue;
        }
        let (path, url, sha1) = if let Some(a) = l["downloads"]["artifact"].as_object() {
            (
                a.get("path").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                a.get("url").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                a.get("sha1").and_then(|x| x.as_str()).map(String::from),
            )
        } else {
            let base = l["url"].as_str().unwrap_or("https://libraries.minecraft.net/");
            let cls = p.get(3).map(|c| format!("-{c}")).unwrap_or_default();
            let path = format!("{}/{}/{}/{}-{}{}.jar", p[0].replace('.', "/"), p[1], p[2], p[1], p[2], cls);
            (path.clone(), format!("{}/{}", base.trim_end_matches('/'), path), l["sha1"].as_str().map(String::from))
        };
        if path.is_empty() || url.is_empty() {
            continue;
        }
        out.push(Lib { path, url, sha1 });
    }
    out
}

/// Download everything a version needs. Returns the (possibly loader-specific) version id.
pub async fn install(app: &AppHandle, mc: &str, loader: Option<&str>, prog: &Prog) -> R<String> {
    let vd = versions_dir(app);
    let root = root(app);
    let vjson = vd.join(mc).join(format!("{mc}.json"));
    if !vjson.exists() {
        prog.stage(&t("stage.versionManifest"));
        let m = get_json(MANIFEST).await?;
        let url = m["versions"]
            .as_array()
            .and_then(|a| a.iter().find(|v| v["id"] == mc))
            .and_then(|v| v["url"].as_str())
            .ok_or_else(|| format!("Unknown Minecraft version {mc}"))?;
        write_json(&vjson, &get_json(url).await?)?;
    }
    let mut id = mc.to_string();
    if let Some(lv) = loader {
        id = format!("fabric-loader-{lv}-{mc}");
        let p = vd.join(&id).join(format!("{id}.json"));
        if !p.exists() {
            prog.stage(&t("stage.fabricProfile"));
            let prof = get_json(&format!("https://meta.fabricmc.net/v2/versions/loader/{mc}/{lv}/profile/json")).await?;
            write_json(&p, &prof)?;
        }
    }
    let resolved = resolve(&vd, &id)?;
    let mut items: Vec<Item> = vec![];
    let client = &resolved["downloads"]["client"];
    items.push(Item {
        url: client["url"].as_str().ok_or("client download missing")?.into(),
        dest: vd.join(mc).join(format!("{mc}.jar")),
        sha1: client["sha1"].as_str().map(String::from),
    });
    for l in libs(&resolved) {
        items.push(Item { url: l.url, dest: root.join("libraries").join(&l.path), sha1: l.sha1 });
    }
    let ai = &resolved["assetIndex"];
    let aid = ai["id"].as_str().ok_or("assetIndex missing")?;
    let idx_path = root.join("assets").join("indexes").join(format!("{aid}.json"));
    prog.stage(&t("stage.libraries"));
    download(&Item {
        url: ai["url"].as_str().unwrap_or_default().into(),
        dest: idx_path.clone(),
        sha1: ai["sha1"].as_str().map(String::from),
    })
    .await?;
    let idx = read_json(&idx_path)?;
    for (_, o) in idx["objects"].as_object().into_iter().flatten() {
        let Some(h) = o["hash"].as_str() else { continue };
        let dest = root.join("assets").join("objects").join(&h[..2]).join(h);
        if dest.exists() {
            continue;
        }
        items.push(Item {
            url: format!("https://resources.download.minecraft.net/{}/{h}", &h[..2]),
            dest,
            sha1: None,
        });
    }
    download_all(items, prog, 24).await?;
    Ok(id)
}

fn sub(s: &str, ctx: &HashMap<String, String>) -> String {
    let mut r = s.to_string();
    for (k, v) in ctx {
        r = r.replace(&format!("${{{k}}}"), v);
    }
    r
}

fn expand(args: &Value, ctx: &HashMap<String, String>, feats: &HashMap<&str, bool>) -> Vec<String> {
    let mut out = vec![];
    for a in args.as_array().into_iter().flatten() {
        match a {
            Value::String(s) => out.push(sub(s, ctx)),
            Value::Object(o) => {
                if let Some(r) = o.get("rules") {
                    if !rules_ok(r, feats) {
                        continue;
                    }
                }
                match &o["value"] {
                    Value::String(s) => out.push(sub(s, ctx)),
                    Value::Array(v) => out.extend(v.iter().filter_map(|x| x.as_str()).map(|s| sub(s, ctx))),
                    _ => {}
                }
            }
            _ => {}
        }
    }
    out
}

pub struct LaunchCfg {
    pub version_id: String,
    pub mc: String,
    pub dir: PathBuf,
    pub server: Option<(String, u16)>,
}

pub async fn launch(app: &AppHandle, cfg: LaunchCfg, prog: &Prog) -> R<u32> {
    let st = settings::load(app);
    let acc = auth::fresh_account(app).await?;
    let vd = versions_dir(app);
    let root = root(app);
    let resolved = resolve(&vd, &cfg.version_id)?;
    let major = resolved["javaVersion"]["majorVersion"].as_u64().unwrap_or(17) as u32;
    let java = java::ensure(app, &st.java_path, major, prog).await?;

    let mut cp: Vec<String> = libs(&resolved)
        .iter()
        .map(|l| root.join("libraries").join(&l.path).to_string_lossy().to_string())
        .collect();
    cp.push(vd.join(&cfg.mc).join(format!("{}.jar", cfg.mc)).to_string_lossy().to_string());
    let natives = vd.join(&cfg.version_id).join("natives");
    std::fs::create_dir_all(&natives).map_err(err)?;
    std::fs::create_dir_all(&cfg.dir).map_err(err)?;

    let has_quickplay = resolved.to_string().contains("quickPlayMultiplayer");
    let mut feats: HashMap<&str, bool> = HashMap::new();
    let mut ctx: HashMap<String, String> = HashMap::new();
    let mut extra_game: Vec<String> = vec![];
    if let Some((host, port)) = &cfg.server {
        if has_quickplay {
            feats.insert("is_quick_play_multiplayer", true);
            ctx.insert("quickPlayMultiplayer".into(), format!("{host}:{port}"));
        } else {
            extra_game.extend(["--server".into(), host.clone(), "--port".into(), port.to_string()]);
        }
    }
    let kv = |k: &str, v: String| (k.to_string(), v);
    ctx.extend([
        kv("auth_player_name", acc.name.clone()),
        kv("version_name", cfg.version_id.clone()),
        kv("game_directory", cfg.dir.to_string_lossy().into()),
        kv("assets_root", root.join("assets").to_string_lossy().into()),
        kv("assets_index_name", resolved["assetIndex"]["id"].as_str().unwrap_or("").into()),
        kv("auth_uuid", acc.uuid.clone()),
        kv("auth_access_token", if acc.kind == "microsoft" { acc.access_token.clone() } else { "0".into() }),
        kv("clientid", String::new()),
        kv("auth_xuid", String::new()),
        kv("user_type", if acc.kind == "microsoft" { "msa" } else { "legacy" }.into()),
        kv("version_type", "release".into()),
        kv("natives_directory", natives.to_string_lossy().into()),
        kv("launcher_name", "lava-se-launcher".into()),
        kv("launcher_version", "0.1".into()),
        kv("classpath", cp.join(";")),
        kv("classpath_separator", ";".into()),
        kv("library_directory", root.join("libraries").to_string_lossy().into()),
    ]);

    let mut jvm: Vec<String> = vec![format!("-Xms{}M", st.ram_min), format!("-Xmx{}M", st.ram_max.max(st.ram_min))];
    jvm.extend(settings::jvm_preset_flags(&st.jvm_preset));
    jvm.extend(st.extra_jvm.split_whitespace().map(String::from));
    jvm.extend(expand(&resolved["arguments"]["jvm"], &ctx, &feats));
    let mut game = expand(&resolved["arguments"]["game"], &ctx, &feats);
    game.extend(extra_game);

    // record the game arguments (token masked) for troubleshooting
    let shown: Vec<String> = game
        .iter()
        .map(|a| if !acc.access_token.is_empty() && *a == acc.access_token { "***".into() } else { a.clone() })
        .collect();
    let _ = std::fs::write(cfg.dir.join("launcher-args.txt"), shown.join("\n"));

    prog.stage(&t("stage.starting"));
    let piped = st.launcher_behavior != "close";
    let mut cmd = tokio::process::Command::new(&java);
    cmd.args(&jvm)
        .arg(resolved["mainClass"].as_str().ok_or("mainClass missing")?)
        .args(&game)
        .current_dir(&cfg.dir)
        .stdin(Stdio::null())
        .stdout(if piped { Stdio::piped() } else { Stdio::null() })
        .stderr(if piped { Stdio::piped() } else { Stdio::null() });
    #[cfg(windows)]
    cmd.creation_flags(0x08000000);
    let mut child = cmd.spawn().map_err(|e| format!("Could not start Java: {e}"))?;
    let pid = child.id().unwrap_or(0);

    for pipe in [
        child.stdout.take().map(|s| Box::new(s) as Box<dyn tokio::io::AsyncRead + Unpin + Send>),
        child.stderr.take().map(|s| Box::new(s) as Box<dyn tokio::io::AsyncRead + Unpin + Send>),
    ]
    .into_iter()
    .flatten()
    {
        let a = app.clone();
        tauri::async_runtime::spawn(async move {
            let mut lines = BufReader::new(pipe).lines();
            while let Ok(Some(l)) = lines.next_line().await {
                let _ = a.emit("game-log", l);
            }
        });
    }
    let a = app.clone();
    let behavior = st.launcher_behavior.clone();
    tauri::async_runtime::spawn(async move {
        let code = child.wait().await.ok().and_then(|s| s.code());
        let _ = a.emit("game-exit", code);
        if behavior == "hide" {
            if let Some(w) = a.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }
    });
    match st.launcher_behavior.as_str() {
        "hide" => {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.hide();
            }
        }
        "close" => {
            tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
            app.exit(0);
        }
        _ => {}
    }
    Ok(pid)
}
