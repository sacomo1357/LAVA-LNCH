//! Text the Rust side shows to the user (errors, progress stages, the login page) in the UI language.
//! The strings live in ../lang/<lang>.json, the same files the frontend reads.

use serde_json::Value;
use std::fmt::Display;
use std::sync::{OnceLock, RwLock};

const FILES: [(&str, &str); 4] = [
    ("ko", include_str!("../../lang/ko.json")),
    ("en", include_str!("../../lang/en.json")),
    ("cn", include_str!("../../lang/cn.json")),
    ("jp", include_str!("../../lang/jp.json")),
];
/// Used for any key a language file leaves out or leaves empty.
const FALLBACK: &str = "ko";

static CURRENT: RwLock<&str> = RwLock::new(FALLBACK);

fn tables() -> &'static Vec<(&'static str, Value)> {
    static T: OnceLock<Vec<(&'static str, Value)>> = OnceLock::new();
    T.get_or_init(|| FILES.iter().map(|(id, src)| (*id, serde_json::from_str(src).unwrap_or(Value::Null))).collect())
}

fn lookup(lang: &str, key: &str) -> Option<&'static str> {
    let (_, root) = tables().iter().find(|(id, _)| *id == lang)?;
    key.split('.').try_fold(root, |v, k| v.get(k))?.as_str().filter(|s| !s.is_empty())
}

pub fn current() -> &'static str {
    *CURRENT.read().unwrap()
}

/// The current language as an HTML `lang` value.
pub fn html_lang() -> &'static str {
    match current() {
        "cn" => "zh-CN",
        "jp" => "ja",
        l => l,
    }
}

pub fn t(key: &str) -> String {
    tf(key, &[])
}

/// `t` with `{name}` placeholders filled in.
pub fn tf(key: &str, args: &[(&str, &dyn Display)]) -> String {
    let mut s = lookup(current(), key).or_else(|| lookup(FALLBACK, key)).unwrap_or(key).to_string();
    for (k, v) in args {
        s = s.replace(&format!("{{{k}}}"), &v.to_string());
    }
    s
}

#[tauri::command]
pub fn set_lang(lang: String) {
    if let Some((id, _)) = FILES.iter().find(|(id, _)| *id == lang) {
        *CURRENT.write().unwrap() = id;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fills_placeholders_and_falls_back() {
        assert!(tf("err.portBusy", &[("port", &9876), ("detail", &"x")]).starts_with("9876"));
        assert_eq!(t("no.such.key"), "no.such.key");
    }
}
