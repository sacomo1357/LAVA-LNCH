import { reactive, watch } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { api, inTauri, type Settings } from './api'
import { defaultBackground } from './pixel'

export type Tab = 'single' | 'multi' | 'skin' | 'setting'
interface Toast { id: number; kind: 'info' | 'success' | 'error' | 'warn'; msg: string }

function pref(key: string, fallback: string): string {
  try { return localStorage.getItem('lava.' + key) ?? fallback } catch { return fallback }
}
function setPref(key: string, v: string) {
  try { localStorage.setItem('lava.' + key, v) } catch { /* storage unavailable */ }
}

export const state = reactive({
  settings: null as Settings | null,
  tab: pref('tab', 'single') as Tab,
  collapsed: pref('collapsed', '0') === '1',
  profile: pref('profile', ''),
  server: pref('server', ''),
  customBg: null as string | null,
  progress: { active: false, label: '', done: 0, total: 0 },
  playing: false,
  log: [] as string[],
  showLog: false,
  toasts: [] as Toast[],
})

watch(() => state.tab, (v) => setPref('tab', v))
watch(() => state.collapsed, (v) => setPref('collapsed', v ? '1' : '0'))
watch(() => state.profile, (v) => setPref('profile', v))
watch(() => state.server, (v) => setPref('server', v))

/** Background for Single (and for servers that don't send their own). */
export const background = () => state.customBg ?? defaultBackground()

let nextId = 1
export function toast(kind: Toast['kind'], msg: string) {
  const id = nextId++
  state.toasts.push({ id, kind, msg })
  setTimeout(() => {
    const i = state.toasts.findIndex((t) => t.id === id)
    if (i >= 0) state.toasts.splice(i, 1)
  }, kind === 'error' ? 8000 : 4000)
}

export async function loadSettings() {
  state.settings = await api.getSettings()
  state.customBg = await api.bgGet().catch(() => null)
}

/** Run an async action, surfacing any backend error as a toast. */
export async function guard<T>(fn: () => Promise<T>): Promise<T | undefined> {
  try {
    return await fn()
  } catch (e) {
    toast('error', String(e))
  }
}

export async function initEvents() {
  if (!inTauri) return
  await listen<{ label: string; done: number; total: number; active: boolean }>('progress', (e) => {
    state.progress = e.payload
  })
  await listen<string>('game-log', (e) => {
    state.log.push(e.payload)
    if (state.log.length > 2000) state.log.splice(0, 500)
  })
  await listen<number | null>('game-exit', (e) => {
    state.playing = false
    state.log.push(`[launcher] 게임 종료 (code ${e.payload ?? '?'})`)
    if (e.payload) toast('warn', `마인크래프트가 code ${e.payload}로 종료됐습니다. 게임 로그를 확인하세요.`)
  })
  await listen<{ kind: Toast['kind']; msg: string }>('toast', (e) => toast(e.payload.kind, e.payload.msg))
}

export async function play(start: () => Promise<number>) {
  if (state.playing) return
  state.log = []
  state.playing = true
  const pid = await guard(start)
  if (pid === undefined) state.playing = false
  else toast('success', '마인크래프트를 시작합니다')
}

/** Read a picked/dropped file as a data URL. */
export function readDataUrl(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const r = new FileReader()
    r.onload = () => resolve(r.result as string)
    r.onerror = () => reject('파일을 읽지 못했습니다')
    r.readAsDataURL(file)
  })
}
