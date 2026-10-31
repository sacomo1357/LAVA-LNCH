use crate::settings::{self, Account, Settings};
use crate::util::*;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64URL, Engine};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, sync::Mutex, time::Duration};
use tauri::AppHandle;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::oneshot,
};

const SCOPE: &str = "XboxLive.signin offline_access";
const AUTHORIZE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize";
const TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
/// Must match the redirect URI registered on the Azure app ("LAVA LNCH", public client/native).
const REDIRECT: &str = "http://localhost:9876";
const PORT: u16 = 9876;

fn offline_uuid(name: &str) -> String {
    let mut h = md5::compute(format!("OfflinePlayer:{name}")).0;
    h[6] = (h[6] & 0x0f) | 0x30;
    h[8] = (h[8] & 0x3f) | 0x80;
    hex::encode(h)
}

#[tauri::command]
pub fn auth_offline(app: AppHandle, name: String) -> R<Settings> {
    let name = name.trim().to_string();
    let ok = (3..=16).contains(&name.len()) && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !ok {
        return Err(t("err.nameInvalid"));
    }
    let mut s = settings::load(&app);
    s.account = Some(Account {
        kind: "offline".into(),
        uuid: offline_uuid(&name),
        name,
        ..Default::default()
    });
    settings::save(&app, &s)?;
    Ok(s)
}

#[tauri::command]
pub fn auth_logout(app: AppHandle) -> R<Settings> {
    let mut s = settings::load(&app);
    s.account = None;
    settings::save(&app, &s)?;
    Ok(s)
}

// ---- Microsoft login: authorization code + PKCE, answered on the loopback redirect ----

/// Cancels the login that is currently waiting for the browser, if any.
static PENDING: Mutex<Option<oneshot::Sender<()>>> = Mutex::new(None);

fn random_b64(n: usize) -> R<String> {
    let mut b = vec![0u8; n];
    getrandom::fill(&mut b).map_err(err)?;
    Ok(B64URL.encode(b))
}

/// Listen on 127.0.0.1 and, when possible, [::1] too: browsers may resolve "localhost" to either.
async fn bind() -> R<Vec<TcpListener>> {
    let mut last = String::new();
    for _ in 0..15 {
        // a cancelled login may still be releasing the port
        match TcpListener::bind(("127.0.0.1", PORT)).await {
            Ok(v4) => {
                let mut v = vec![v4];
                if let Ok(v6) = TcpListener::bind(("::1", PORT)).await {
                    v.push(v6);
                }
                return Ok(v);
            }
            Err(e) => last = e.to_string(),
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    Err(tf("err.portBusy", &[("port", &PORT), ("detail", &last)]))
}

async fn accept(ls: &[TcpListener]) -> std::io::Result<TcpStream> {
    if ls.len() > 1 {
        tokio::select! {
            r = ls[0].accept() => r.map(|x| x.0),
            r = ls[1].accept() => r.map(|x| x.0),
        }
    } else {
        ls[0].accept().await.map(|x| x.0)
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

async fn reply(mut sock: TcpStream, status: &str, title: &str, msg: &str, ok: bool) {
    let body = format!(
        r#"<!doctype html><html lang="{lang}"><meta charset="utf-8"><title>LAVA LNCH</title>
<body style="margin:0;height:100vh;display:grid;place-items:center;background:#0f1115;color:#e6e8ee;font-family:'Segoe UI','Malgun Gothic',sans-serif">
<div style="text-align:center;padding:32px 40px;background:#1b1f27;box-shadow:0 0 0 3px #343b4d,6px 6px 0 3px rgba(0,0,0,.3)">
<div style="font-weight:700;letter-spacing:.16em;color:#ff6a2b;margin-bottom:14px">LAVA LNCH</div>
<h2 style="margin:0 0 8px;color:{}">{}</h2><p style="margin:0;color:#8b93a7">{}</p></div></body></html>"#,
        if ok { "#3ccf91" } else { "#ff5c5c" },
        escape(title),
        escape(msg),
        lang = crate::i18n::html_lang()
    );
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = sock.write_all(head.as_bytes()).await;
    let _ = sock.write_all(body.as_bytes()).await;
    let _ = sock.shutdown().await;
}

/// Serve the redirect until Microsoft sends back a code (or an error) for our `state`.
async fn wait_for_code(ls: &[TcpListener], state: &str) -> R<String> {
    loop {
        let mut sock = accept(ls).await.map_err(err)?;
        let mut buf = vec![0u8; 16 * 1024];
        let n = match tokio::time::timeout(Duration::from_secs(5), sock.read(&mut buf)).await {
            Ok(Ok(n)) => n,
            _ => continue,
        };
        let req = String::from_utf8_lossy(&buf[..n]);
        let path = req.split_whitespace().nth(1).unwrap_or("/");
        let Ok(url) = reqwest::Url::parse(&format!("http://localhost{path}")) else { continue };
        let q: HashMap<String, String> = url.query_pairs().into_owned().collect();
        if let Some(code) = q.get("code") {
            if q.get("state").map(String::as_str) != Some(state) {
                reply(sock, "400 Bad Request", &t("login.failTitle"), &t("login.mismatch"), false).await;
                return Err(t("err.stateMismatch"));
            }
            reply(sock, "200 OK", &t("login.doneTitle"), &t("login.doneMsg"), true).await;
            return Ok(code.clone());
        }
        if let Some(e) = q.get("error") {
            let desc = q.get("error_description").cloned().unwrap_or_else(|| e.clone());
            let desc = desc.lines().next().unwrap_or(&desc).to_string();
            reply(sock, "200 OK", &t("login.failTitle"), &desc, false).await;
            return Err(if e == "access_denied" { t("err.msCancelled") } else { desc });
        }
        // favicon and other stray requests
        reply(sock, "404 Not Found", "LAVA LNCH", "", false).await;
    }
}

/// Open the Microsoft sign-in page in the browser and finish the login when it redirects back.
#[tauri::command]
pub async fn auth_ms_login(app: AppHandle) -> R<Settings> {
    let mut s = settings::load(&app);
    let client_id = s.client_id();
    if client_id.is_empty() {
        return Err(t("err.clientIdMissing"));
    }
    let (tx, mut cancel) = oneshot::channel();
    if let Some(old) = PENDING.lock().unwrap().replace(tx) {
        let _ = old.send(());
    }
    let listeners = bind().await?;
    let verifier = random_b64(32)?;
    let challenge = B64URL.encode(Sha256::digest(verifier.as_bytes()));
    let state = random_b64(16)?;
    let url = reqwest::Url::parse_with_params(
        AUTHORIZE_URL,
        &[
            ("client_id", client_id.as_str()),
            ("response_type", "code"),
            ("redirect_uri", REDIRECT),
            ("response_mode", "query"),
            ("scope", SCOPE),
            ("code_challenge", challenge.as_str()),
            ("code_challenge_method", "S256"),
            ("state", state.as_str()),
            ("prompt", "select_account"),
        ],
    )
    .map_err(err)?;
    settings::open_url(url.to_string())?;

    let code = tokio::select! {
        r = wait_for_code(&listeners, &state) => r?,
        _ = &mut cancel => return Err(t("err.loginCancelled")),
        _ = tokio::time::sleep(Duration::from_secs(300)) => return Err(t("err.loginTimeout")),
    };
    drop(listeners);
    PENDING.lock().unwrap().take();

    let v: Value = http()
        .post(TOKEN_URL)
        .form(&[
            ("client_id", client_id.as_str()),
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", REDIRECT),
            ("code_verifier", verifier.as_str()),
            ("scope", SCOPE),
        ])
        .send()
        .await
        .map_err(err)?
        .json()
        .await
        .map_err(err)?;
    let Some(at) = v["access_token"].as_str() else {
        let Some(m) = v["error_description"].as_str() else { return Err(t("err.noToken")) };
        return Err(m.lines().next().unwrap_or(m).to_string());
    };
    s.account = Some(login_chain(at, v["refresh_token"].as_str().unwrap_or("")).await?);
    settings::save(&app, &s)?;
    Ok(s)
}

#[tauri::command]
pub fn auth_ms_cancel() {
    if let Some(tx) = PENDING.lock().unwrap().take() {
        let _ = tx.send(());
    }
}

async fn post_json(url: &str, body: Value) -> R<Value> {
    let r = http()
        .post(url)
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(err)?;
    let status = r.status();
    let v: Value = r.json().await.unwrap_or(Value::Null);
    if !status.is_success() {
        if let Some(x) = v["XErr"].as_u64() {
            let hint = match x {
                2148916233 => t("err.xboxNoProfile"),
                2148916235 => t("err.xboxRegion"),
                2148916238 => t("err.xboxChild"),
                _ => t("err.xboxFailed"),
            };
            return Err(format!("{hint} (XErr {x})"));
        }
        if url.contains("login_with_xbox") && status.as_u16() == 403 {
            return Err(t("err.mcApiNotApproved"));
        }
        return Err(format!("{url} -> {status}"));
    }
    Ok(v)
}

/// Microsoft access token -> Xbox Live -> XSTS -> Minecraft services.
pub async fn login_chain(ms_token: &str, refresh: &str) -> R<Account> {
    let xbl = post_json(
        "https://user.auth.xboxlive.com/user/authenticate",
        json!({
            "Properties": {"AuthMethod": "RPS", "SiteName": "user.auth.xboxlive.com", "RpsTicket": format!("d={ms_token}")},
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        }),
    )
    .await?;
    let xbl_token = xbl["Token"].as_str().ok_or_else(|| t("err.noXblToken"))?;
    let xsts = post_json(
        "https://xsts.auth.xboxlive.com/xsts/authorize",
        json!({
            "Properties": {"SandboxId": "RETAIL", "UserTokens": [xbl_token]},
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        }),
    )
    .await?;
    let xsts_token = xsts["Token"].as_str().ok_or_else(|| t("err.noXstsToken"))?;
    let uhs = xsts["DisplayClaims"]["xui"][0]["uhs"].as_str().ok_or_else(|| t("err.noUhs"))?;
    let mc = post_json(
        "https://api.minecraftservices.com/authentication/login_with_xbox",
        json!({"identityToken": format!("XBL3.0 x={uhs};{xsts_token}")}),
    )
    .await?;
    let token = mc["access_token"].as_str().ok_or_else(|| t("err.noMcToken"))?.to_string();
    let expires = mc["expires_in"].as_u64().unwrap_or(3600);
    let prof = http()
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(&token)
        .send()
        .await
        .map_err(err)?;
    if prof.status().as_u16() == 404 {
        return Err(t("err.noJavaEdition"));
    }
    let prof: Value = prof.error_for_status().map_err(err)?.json().await.map_err(err)?;
    Ok(Account {
        kind: "microsoft".into(),
        name: prof["name"].as_str().unwrap_or_default().into(),
        uuid: prof["id"].as_str().unwrap_or_default().into(),
        access_token: token,
        refresh_token: refresh.to_string(),
        expires_at: now() + expires,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Play the browser: hit the redirect the way Microsoft would after sign-in.
    async fn redirect(path: &str) -> String {
        let mut s = TcpStream::connect(("127.0.0.1", PORT)).await.unwrap();
        s.write_all(format!("GET {path} HTTP/1.1\r\nHost: localhost:{PORT}\r\n\r\n").as_bytes()).await.unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).await.unwrap();
        out
    }

    // one test so the three cases never fight over the port
    #[tokio::test]
    async fn loopback_redirect() {
        // stray request first, then the real code
        let ls = bind().await.unwrap();
        let browser = tokio::spawn(async {
            let fav = redirect("/favicon.ico").await;
            let ok = redirect("/?code=M.C5_abc%2Bdef&state=xyz").await;
            (fav, ok)
        });
        assert_eq!(wait_for_code(&ls, "xyz").await.unwrap(), "M.C5_abc+def");
        let (fav, ok) = browser.await.unwrap();
        assert!(fav.starts_with("HTTP/1.1 404"));
        assert!(ok.starts_with("HTTP/1.1 200") && ok.contains("로그인 완료"));
        drop(ls);

        // a code for someone else's state is rejected
        let ls = bind().await.unwrap();
        let browser = tokio::spawn(redirect("/?code=evil&state=other"));
        assert!(wait_for_code(&ls, "xyz").await.unwrap_err().contains("state"));
        assert!(browser.await.unwrap().starts_with("HTTP/1.1 400"));
        drop(ls);

        // the user cancels on the Microsoft page; the description is escaped in the reply
        let ls = bind().await.unwrap();
        let browser = tokio::spawn(redirect("/?error=access_denied&error_description=%3Cb%3Eno%3C%2Fb%3E&state=xyz"));
        assert!(wait_for_code(&ls, "xyz").await.unwrap_err().contains("취소"));
        let page = browser.await.unwrap();
        assert!(page.contains("&lt;b&gt;no") && !page.contains("<b>no"));
    }

    #[test]
    fn pkce_challenge_matches_rfc7636_example() {
        let v = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert_eq!(B64URL.encode(Sha256::digest(v.as_bytes())), "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }
}

/// The logged-in account with a token that is valid for at least another minute.
pub async fn fresh_account(app: &AppHandle) -> R<Account> {
    let mut s = settings::load(app);
    let acc = s.account.clone().ok_or_else(|| t("err.loginFirst"))?;
    if acc.kind != "microsoft" || acc.expires_at > now() + 60 {
        return Ok(acc);
    }
    let v: Value = http()
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", s.client_id().as_str()),
            ("refresh_token", acc.refresh_token.as_str()),
            ("redirect_uri", REDIRECT),
            ("scope", SCOPE),
        ])
        .send()
        .await
        .map_err(err)?
        .json()
        .await
        .map_err(err)?;
    let at = v["access_token"].as_str().ok_or_else(|| t("err.loginExpired"))?;
    let rt = v["refresh_token"].as_str().unwrap_or(&acc.refresh_token);
    let new = login_chain(at, rt).await?;
    s.account = Some(new.clone());
    settings::save(app, &s)?;
    Ok(new)
}
