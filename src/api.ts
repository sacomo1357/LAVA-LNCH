import { invoke as tauriInvoke } from '@tauri-apps/api/core'

export interface Account {
  kind: 'offline' | 'microsoft'
  name: string
  uuid: string
  access_token: string
  refresh_token: string
  expires_at: number
}
export interface Settings {
  java_path: string
  ram_min: number
  ram_max: number
  jvm_preset: string
  extra_jvm: string
  launcher_behavior: string
  ms_client_id: string
  last_server: string
  account: Account | null
  update_repo: string
  auto_update: boolean
}
export interface UpdateInfo {
  current: string
  latest: string
  available: boolean
  can_install: boolean
  repo: string
  notes: string
  page: string
  asset_name: string
  asset_url: string
  size: number
}
export interface Instance {
  name: string
  mc: string
  loader: string
  loader_version: string
  kind: string
  server: string
  version_id: string
  address: string
  title: string
  description: string
  icon: string
  background: string
}
export interface JavaInfo { path: string; major: number; version: string }
export interface ModFile { file: string; enabled: boolean; size: number }
export interface SkinData { png_b64: string | null; variant: 'classic' | 'slim'; source: string }
export interface SkinEntry { id: string; name: string; variant: 'classic' | 'slim'; added: number; png_b64: string }
export interface ModHit { project_id: string; slug: string; title: string; description: string; downloads: number; icon_url: string | null; author: string }

export const inTauri = '__TAURI_INTERNALS__' in window

// Lets the UI run in a plain browser (npm run dev) for layout work.
const mockSettings: Settings = {
  java_path: '', ram_min: 1024, ram_max: 4096, jvm_preset: 'aikar', extra_jvm: '',
  launcher_behavior: 'hide', ms_client_id: '', last_server: '', account: null,
  update_repo: '', auto_update: true,
}
const blank: Instance = { name: '', mc: '', loader: 'fabric', loader_version: '0.16.7', kind: 'custom', server: '', version_id: 'x', address: '', title: '', description: '', icon: '', background: '' }
const mockInst: Instance[] = [
  { ...blank, name: 'Lava_Optimized', mc: '1.21.1', kind: 'preset' },
  { ...blank, name: 'Vanilla_1.21', mc: '1.21', loader: 'vanilla', loader_version: '' },
]
const mockSkins: SkinEntry[] = []
const mocks: Record<string, (a: any) => unknown> = {
  get_settings: () => ({ ...mockSettings }),
  save_settings: (a) => ({ ...a.settings }),
  auth_offline: (a) => (mockSettings.account = { kind: 'offline', name: a.name, uuid: '0', access_token: '', refresh_token: '', expires_at: 0 }, { ...mockSettings }),
  auth_logout: () => (mockSettings.account = null, { ...mockSettings }),
  list_instances: () => mockInst.map((i) => ({ ...i })),
  mc_versions: () => ({ latest: '1.21.1', versions: ['1.21.1', '1.21', '1.20.6', '1.20.4', '1.19.4'] }),
  list_mods: () => [
    { file: 'fabric-api-0.102.0.jar', enabled: true, size: 2_400_000 },
    { file: 'sodium-fabric-0.6.0.jar', enabled: true, size: 1_542_300 },
    { file: 'iris-1.8.0.jar', enabled: false, size: 1_900_000 },
  ],
  modrinth_search: () => ({ hits: [{ project_id: 'AANobbMI', slug: 'sodium', title: 'Sodium', description: 'The fastest and most compatible rendering optimization mod.', downloads: 40_000_000, icon_url: null, author: 'jellysquid3' }], total_hits: 1 }),
  java_detect: () => [{ path: 'C:\\Program Files\\Eclipse Adoptium\\jdk-21\\bin\\java.exe', major: 21, version: '21.0.4' }],
  skin_get: () => ({ png_b64: null, variant: 'classic', source: 'none' }),
  skin_list: () => mockSkins.map((s) => ({ ...s })),
  skin_add: (a) => { const e = { id: Date.now().toString(16), name: a.name, variant: a.variant, added: 0, png_b64: a.pngB64 }; mockSkins.push(e); return e },
  skin_delete: (a) => { mockSkins.splice(mockSkins.findIndex((s) => s.id === a.id), 1) },
  skin_apply: () => '적용했습니다 (미리보기)',
  bg_get: () => null,
  builtin_client_id: () => false,
  update_default_repo: () => 'sacomo1357/LAVA-LNCH',
  update_check: () => ({ current: '0.1.0', latest: '0.2.0', available: new URLSearchParams(location.search).has('update'), can_install: true, repo: 'sacomo1357/LAVA-LNCH', notes: '- 새 기능\n- 버그 수정', page: '', asset_name: 'LAVA.SE.Launcher_0.2.0_x64-setup.exe', asset_url: '', size: 3_300_000 }),
  bg_set: () => null,
  server_add: (a) => {
    const host = String(a.address).replace(/^https?:\/\//, '').split(/[/:]/)[0]
    const i = { ...blank, name: 'srv-' + host, mc: '1.21.1', kind: 'server', server: host + ':25565', address: a.address, title: host }
    mockInst.push(i)
    return i
  },
  delete_instance: (a) => { mockInst.splice(mockInst.findIndex((i) => i.name === a.name), 1) },
}

export async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!inTauri) {
    const m = mocks[cmd]
    if (m) return m(args ?? {}) as T
    throw '브라우저 미리보기에서는 사용할 수 없습니다'
  }
  return tauriInvoke<T>(cmd, args)
}

export const api = {
  getSettings: () => call<Settings>('get_settings'),
  saveSettings: (settings: Settings) => call<Settings>('save_settings', { settings }),
  openFolder: (sub = '') => call<void>('open_folder', { sub }),
  openUrl: (url: string) => call<void>('open_url', { url }),
  bgGet: () => call<string | null>('bg_get'),
  bgSet: (dataUrl: string | null) => call<void>('bg_set', { dataUrl }),
  javaDetect: () => call<JavaInfo[]>('java_detect'),
  javaInstall: (major: number) => call<string>('java_install', { major }),
  authOffline: (name: string) => call<Settings>('auth_offline', { name }),
  authLogout: () => call<Settings>('auth_logout'),
  builtinClientId: () => call<boolean>('builtin_client_id'),
  updateCheck: () => call<UpdateInfo>('update_check'),
  updateInstall: () => call<void>('update_install'),
  updateDefaultRepo: () => call<string>('update_default_repo'),
  authMsLogin: () => call<Settings>('auth_ms_login'),
  authMsCancel: () => call<void>('auth_ms_cancel'),
  mcVersions: () => call<{ latest: string; versions: string[] }>('mc_versions'),
  listInstances: () => call<Instance[]>('list_instances'),
  createInstance: (name: string, mc: string, fabric: boolean, preset: boolean) => call<Instance>('create_instance', { name, mc, fabric, preset }),
  deleteInstance: (name: string) => call<void>('delete_instance', { name }),
  launchInstance: (name: string) => call<number>('launch_instance', { name }),
  listMods: (name: string) => call<ModFile[]>('list_mods', { name }),
  toggleMod: (name: string, file: string) => call<void>('toggle_mod', { name, file }),
  deleteMod: (name: string, file: string) => call<void>('delete_mod', { name, file }),
  modrinthSearch: (query: string, mc: string, offset = 0) => call<{ hits: ModHit[]; total_hits: number }>('modrinth_search', { query, mc, offset }),
  modrinthInstall: (name: string, projectId: string) => call<string[]>('modrinth_install', { name, projectId }),
  serverAdd: (address: string) => call<Instance>('server_add', { address }),
  multiPlay: (address: string) => call<number>('multi_play', { address }),
  skinGet: () => call<SkinData>('skin_get'),
  skinList: () => call<SkinEntry[]>('skin_list'),
  skinAdd: (name: string, pngB64: string, variant: string) => call<SkinEntry>('skin_add', { name, pngB64, variant }),
  skinDelete: (id: string) => call<void>('skin_delete', { id }),
  skinApply: (id: string) => call<string>('skin_apply', { id }),
}
