# LAVA SE Launcher

Minecraft launcher built with **Tauri 2 + Vue 3**, styled after the LAVA SE web panel (Galmuri pixel font, lava-orange theme).

## Tabs

| Tab | What it does |
| --- | --- |
| **Single** | Profiles. One click creates the *Lava preset* (Fabric + Fabric API, Sodium, Iris, Lithium, FerriteCore). Search Modrinth, install mods with required dependencies, toggle or remove mods. |
| **Multi** | Enter a server domain. The launcher fetches the server's manifest, downloads missing/changed files (SHA-1 checked, 8 in parallel), deletes unlisted files in `mods/`, `resourcepacks/`, `shaderpacks/`, installs the right Minecraft + Fabric, then starts the game straight into the server. |
| **Skin** | 3D preview (Three.js) with drag-to-rotate, PNG drag & drop, Classic/Slim auto-detect. Microsoft accounts upload to Mojang; offline accounts keep the skin locally. |
| **Setting** | Account, background image, Java detect / auto-install (Temurin), RAM sliders, GC preset (Aikar / ZGC / Shenandoah), updates, launcher behaviour (hide / keep / close). |

## Develop

```bash
npm install
npm run tauri dev     # app + hot reload
npm run tauri build   # NSIS + MSI installers in src-tauri/target/release/bundle
npm run dev           # UI only in a browser, with mock data
```

Needs Node 20+, Rust (stable), and the MSVC build tools + WebView2 on Windows.

Game data lives in `%APPDATA%\com.lavase.launcher\lava\` (`instances/`, `versions/`, `libraries/`, `assets/`, `runtime/`, `settings.json`).

## Releases and auto-update

1. Bump `version` in `src-tauri/tauri.conf.json` (e.g. `0.2.0`).
2. Commit, then push a matching tag: `git tag v0.2.0 && git push origin v0.2.0`.
3. GitHub Actions (`.github/workflows/release.yml`) builds the NSIS + MSI installers and publishes the release.

On every start the launcher reads the latest release of its update repo. If the tag is a higher version than the running app, it shows a short countdown, downloads the `-setup.exe`, closes, installs it quietly and starts the new version. Turn this off or check manually in Setting → 업데이트. Development builds only report updates and never install them.

### Using your own release link

The update source lives in one file, **`launcher.config.json`**:

```json
{ "update_repo": "sacomo1357/LAVA-LNCH" }
```

Change it to your fork (`owner/repo` or a `https://github.com/owner/repo` URL) and rebuild. Users can also override it per machine in Setting → 업데이트 → 릴리즈 저장소.

## Microsoft login

Browser sign-in (authorization code + PKCE) for the Azure app **LAVA LNCH**, whose client ID is built in (`DEFAULT_MS_CLIENT_ID` in `src-tauri/src/settings.rs`). The app's redirect URI must be `http://localhost:9876` (public client / native); the launcher listens there while you sign in. A new Azure app must be approved for the Minecraft services API (https://aka.ms/mce-reviewappid) before logins succeed. Using your own app? Put its client ID in Setting or change the constant.

## Server manifest

Serve this at `https://<domain>/launcher/manifest.json` (also tried: `https://api.<domain>/launcher/manifest.json`, then plain `http://` for both). You can also type a full `https://.../manifest.json` URL as the address.

```json
{
  "name": "My Server",
  "description": "Shown under the server name",
  "icon": "icon.png",
  "background": "https://cdn.example.com/launcher-bg.jpg",
  "minecraft_version": "1.21.1",
  "loader": { "type": "fabric", "version": "0.16.7" },
  "required_files": [
    { "path": "mods/sodium-fabric-0.6.0.jar", "sha1": "…", "url": "https://cdn.example.com/mods/sodium-fabric-0.6.0.jar", "size": 1542300 }
  ]
}
```

* `name`, `description`, `icon`, `background` are optional. `icon`/`background` may be absolute URLs or paths relative to the manifest. Without an icon the launcher draws a creeper face in a colour derived from the server address; without a background it uses the launcher's own (Setting → 배경화면).
* `path` must be relative with `/` separators, no `..`.
* Files in `mods/`, `resourcepacks/` and `shaderpacks/` that are not listed are deleted on every sync. Other folders (e.g. `config/`) are never cleaned.
* `loader` may be omitted or `vanilla` for an unmodded server. Only `fabric` is supported for mods.

## Limits

* Minecraft 1.19 and newer only (older versions need native extraction).
* Forge/NeoForge/Quilt are not supported yet.
* Capes are not shown or changed.

## License notes

The Galmuri pixel fonts in `public/fonts` are © Lee Minseo, under the SIL Open Font License 1.1 (`public/fonts/Galmuri-OFL.md`).

## Languages

The UI follows the OS language on first start (ko, en, cn = 中文, jp = 日本語; anything else gets en). Setting → 런처 → 언어 changes it.

- **Text:** `lang/<lang>.json`, shared by the Vue UI (`src/i18n.ts`, `t('key', { name })`) and the Rust side (`src-tauri/src/i18n.rs`, `t` / `tf` for errors, progress and the login page).
- `ko.json` is the master. Every key another file leaves out or leaves empty shows the Korean text, so a half-translated file is fine.
- `{name}` placeholders are filled at run time; keep them as they are when translating.
