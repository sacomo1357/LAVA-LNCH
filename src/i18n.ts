import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import ko from '../lang/ko.json'
import en from '../lang/en.json'
import cn from '../lang/cn.json'
import jp from '../lang/jp.json'

// Strings live in ../lang/<lang>.json (shared with the Rust side). ko is the master copy:
// any key another file leaves out or leaves empty shows the Korean text.
export type Lang = 'ko' | 'en' | 'cn' | 'jp'
export const LANGS: { id: Lang; label: string }[] = [
  { id: 'ko', label: '한국어' },
  { id: 'en', label: 'English' },
  { id: 'cn', label: '简体中文' },
  { id: 'jp', label: '日本語' },
]
const tables: Record<Lang, unknown> = { ko, en, cn, jp }

/** The OS / browser language mapped to one we ship; English for everything else. */
function detect(): Lang {
  const l = (navigator.languages?.[0] ?? navigator.language ?? '').toLowerCase()
  if (l.startsWith('ko')) return 'ko'
  if (l.startsWith('zh')) return 'cn'
  if (l.startsWith('ja')) return 'jp'
  return 'en'
}
function saved(): Lang | null {
  try {
    const v = localStorage.getItem('lava.lang')
    return v && v in tables ? (v as Lang) : null
  } catch { return null }
}

export const lang = ref<Lang>(saved() ?? detect())

function apply(l: Lang) {
  document.documentElement.lang = l === 'cn' ? 'zh-CN' : l === 'jp' ? 'ja' : l
  if ('__TAURI_INTERNALS__' in window) invoke('set_lang', { lang: l }).catch(() => {})
}
apply(lang.value)
watch(lang, (l) => {
  try { localStorage.setItem('lava.lang', l) } catch { /* storage unavailable */ }
  apply(l)
})

function lookup(table: unknown, key: string): string | undefined {
  const v = key.split('.').reduce<any>((o, k) => o?.[k], table)
  return typeof v === 'string' && v ? v : undefined
}

/**
 * Release notes can carry one block per language, each starting with a marker like
 * <!-- en --> (invisible on GitHub). Returns the current language's block, else ko's,
 * else the whole text for notes without markers.
 */
export function pickNotes(text: string): string {
  const parts = text.split(/<!--\s*(ko|en|cn|jp)\s*-->/)
  if (parts.length < 3) return parts[0].trim()
  const by: Record<string, string> = {}
  for (let i = 1; i < parts.length; i += 2) by[parts[i]] = parts[i + 1].trim()
  return by[lang.value] || by.ko || parts[0].trim()
}

/** Text for `key` in the current language, with `{name}` placeholders filled from `params`. */
export function t(key: string, params?: Record<string, string | number>): string {
  const s = lookup(tables[lang.value], key) ?? lookup(ko, key) ?? key
  return params ? s.replace(/\{(\w+)\}/g, (m, k) => (k in params ? String(params[k]) : m)) : s
}
