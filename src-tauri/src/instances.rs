use crate::mc::{self, LaunchCfg};
use crate::util::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashSet, path::PathBuf};
use tauri::AppHandle;

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Instance {
    pub name: String,
    pub mc: String,
    pub loader: String, // "fabric" | "vanilla"
    pub loader_version: String,
    pub kind: String, // "preset" | "custom" | "server"
    pub server: String,
    pub version_id: String,
    // server profiles only: what the user typed, plus display info from the manifest
    pub address: String,
    pub title: String,
    pub description: String,
    pub icon: String,
    pub background: String,
}

pub fn inst_dir(app: &AppHandle, name: &str) -> PathBuf {
    root(app).join("instances").join(name)
}

pub fn load_inst(app: &AppHandle, name: &str) -> R<Instance> {
    let b = std::fs::read(inst_dir(app, name).join("instance.json")).map_err(|_| format!("Instance '{name}' not found"))?;
    serde_json::from_slice(&b).map_err(err)
}

pub fn save_inst(app: &AppHandle, i: &Instance) -> R<()> {
    let d = inst_dir(app, &i.name);
    std::fs::create_dir_all(&d).map_err(err)?;
    std::fs::write(d.join("instance.json"), serde_json::to_vec_pretty(i).map_err(err)?).map_err(err)
}

#[tauri::command]
pub fn list_instances(app: AppHandle) -> Vec<Instance> {
    let mut v: Vec<Instance> = std::fs::read_dir(root(&app).join("instances"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| std::fs::read(e.path().join("instance.json")).ok())
        .filter_map(|b| serde_json::from_slice(&b).ok())
        .collect();
    v.sort_by(|a, b| a.name.cmp(&b.name));
    v
}

const PRESET_MODS: [&str; 5] = ["fabric-api", "sodium", "iris", "lithium", "ferrite-core"];

#[tauri::command]
pub async fn create_instance(app: AppHandle, name: String, mc: String, fabric: bool, preset: bool) -> R<Instance> {
    let name = sanitize(&name);
    if name.is_empty() {
        return Err("Enter a profile name".into());
    }
    if inst_dir(&app, &name).join("instance.json").exists() {
        return Err(format!("A profile named '{name}' already exists"));
    }
    let prog = Prog::new(&app);
    let r = async {
        let fabric = fabric || preset;
        let lv = if fabric { Some(mc::latest_loader().await?) } else { None };
        let id = mc::install(&app, &mc, lv.as_deref(), &prog).await?;
        let inst = Instance {
            name: name.clone(),
            mc: mc.clone(),
            loader: if fabric { "fabric".into() } else { "vanilla".into() },
            loader_version: lv.unwrap_or_default(),
            kind: if preset { "preset".into() } else { "custom".into() },
            version_id: id,
            ..Default::default()
        };
        save_inst(&app, &inst)?;
        if preset {
            prog.stage(&t("stage.presetMods"));
            let mut seen = HashSet::new();
            for slug in PRESET_MODS {
                if let Err(e) = install_mod(&app, &inst, slug, &mut seen, &prog).await {
                    emit_toast(&app, "warn", &format!("{slug}: {e}"));
                }
            }
        }
        Ok(inst)
    }
    .await;
    prog.finish();
    if r.is_err() {
        let _ = std::fs::remove_dir_all(inst_dir(&app, &name));
    }
    r
}

pub fn emit_toast(app: &AppHandle, kind: &str, msg: &str) {
    use tauri::Emitter;
    let _ = app.emit("toast", serde_json::json!({"kind": kind, "msg": msg}));
}

#[tauri::command]
pub fn delete_instance(app: AppHandle, name: String) -> R<()> {
    if !safe_name(&name) {
        return Err("bad name".into());
    }
    std::fs::remove_dir_all(inst_dir(&app, &name)).map_err(err)
}

#[tauri::command]
pub async fn launch_instance(app: AppHandle, name: String) -> R<u32> {
    if !safe_name(&name) {
        return Err("bad name".into());
    }
    let inst = load_inst(&app, &name)?;
    let prog = Prog::new(&app);
    let r = async {
        let lv = (inst.loader == "fabric").then_some(inst.loader_version.as_str());
        let id = mc::install(&app, &inst.mc, lv, &prog).await?;
        mc::launch(&app, LaunchCfg { version_id: id, mc: inst.mc.clone(), dir: inst_dir(&app, &name), server: None }, &prog).await
    }
    .await;
    prog.finish();
    r
}

// ---- mods ----

#[derive(Serialize)]
pub struct ModFile {
    file: String,
    enabled: bool,
    size: u64,
}

#[tauri::command]
pub fn list_mods(app: AppHandle, name: String) -> R<Vec<ModFile>> {
    if !safe_name(&name) {
        return Err("bad name".into());
    }
    let mut v: Vec<ModFile> = std::fs::read_dir(inst_dir(&app, &name).join("mods"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let f = e.file_name().to_string_lossy().to_string();
            let enabled = f.ends_with(".jar");
            (enabled || f.ends_with(".jar.disabled")).then(|| ModFile {
                file: f,
                enabled,
                size: e.metadata().map(|m| m.len()).unwrap_or(0),
            })
        })
        .collect();
    v.sort_by(|a, b| a.file.to_lowercase().cmp(&b.file.to_lowercase()));
    Ok(v)
}

#[tauri::command]
pub fn toggle_mod(app: AppHandle, name: String, file: String) -> R<()> {
    if !safe_name(&name) || !safe_name(&file) {
        return Err("bad name".into());
    }
    let d = inst_dir(&app, &name).join("mods");
    let to = match file.strip_suffix(".disabled") {
        Some(f) => f.to_string(),
        None => format!("{file}.disabled"),
    };
    std::fs::rename(d.join(&file), d.join(to)).map_err(err)
}

#[tauri::command]
pub fn delete_mod(app: AppHandle, name: String, file: String) -> R<()> {
    if !safe_name(&name) || !safe_name(&file) {
        return Err("bad name".into());
    }
    std::fs::remove_file(inst_dir(&app, &name).join("mods").join(file)).map_err(err)
}

#[tauri::command]
pub async fn modrinth_search(query: String, mc: String, offset: u32) -> R<Value> {
    let facets = format!(r#"[["project_type:mod"],["categories:fabric"],["versions:{mc}"]]"#);
    get_json_q(
        "https://api.modrinth.com/v2/search",
        &[("query", &query), ("facets", &facets), ("limit", "20"), ("offset", &offset.to_string())],
    )
    .await
}

/// Install a Modrinth project plus its required dependencies into the instance's mods folder.
async fn install_mod(app: &AppHandle, inst: &Instance, project: &str, seen: &mut HashSet<String>, prog: &Prog) -> R<Vec<String>> {
    let mut queue = vec![project.to_string()];
    let mut files = vec![];
    let mods = inst_dir(app, &inst.name).join("mods");
    while let Some(p) = queue.pop() {
        if !seen.insert(p.clone()) {
            continue;
        }
        let loaders = r#"["fabric"]"#;
        let gv = format!(r#"["{}"]"#, inst.mc);
        let vs = get_json_q(
            &format!("https://api.modrinth.com/v2/project/{p}/version"),
            &[("loaders", loaders), ("game_versions", &gv)],
        )
        .await?;
        let v = vs.as_array().and_then(|a| a.first()).ok_or_else(|| format!("no Fabric build for {}", inst.mc))?;
        let f = v["files"]
            .as_array()
            .and_then(|a| a.iter().find(|f| f["primary"] == true).or_else(|| a.first()))
            .ok_or("version has no files")?;
        let fname = f["filename"].as_str().unwrap_or_default().to_string();
        if !safe_name(&fname) {
            return Err("unsafe file name from Modrinth".into());
        }
        download_all(
            vec![Item {
                url: f["url"].as_str().unwrap_or_default().into(),
                dest: mods.join(&fname),
                sha1: f["hashes"]["sha1"].as_str().map(String::from),
            }],
            prog,
            1,
        )
        .await?;
        files.push(fname);
        for d in v["dependencies"].as_array().into_iter().flatten() {
            if d["dependency_type"] == "required" {
                if let Some(id) = d["project_id"].as_str() {
                    queue.push(id.to_string());
                }
            }
        }
    }
    Ok(files)
}

#[tauri::command]
pub async fn modrinth_install(app: AppHandle, name: String, project_id: String) -> R<Vec<String>> {
    if !safe_name(&name) {
        return Err("bad name".into());
    }
    let inst = load_inst(&app, &name)?;
    if inst.loader != "fabric" {
        return Err("This profile has no mod loader. Create a Fabric profile to use mods.".into());
    }
    let prog = Prog::new(&app);
    prog.stage(&t("stage.mod"));
    let r = install_mod(&app, &inst, &project_id, &mut HashSet::new(), &prog).await;
    prog.finish();
    r
}
