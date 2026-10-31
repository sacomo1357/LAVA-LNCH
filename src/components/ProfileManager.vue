<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { api, type Instance, type ModFile, type ModHit } from '../api'
import { guard, state, toast } from '../store'
import { t } from '../i18n'
import PxIcon from './PxIcon.vue'

const props = defineProps<{ startNew: boolean; initial: string }>()
const emit = defineEmits<{ close: [] }>()

const instances = ref<Instance[]>([])
const selected = ref(props.startNew ? '' : props.initial) // '' = new-profile form
const mods = ref<ModFile[]>([])
const query = ref('')
const hits = ref<ModHit[]>([])
const searching = ref(false)
const installing = ref('')
const versions = ref<string[]>([])
const form = ref({ name: '', mc: '', fabric: true, preset: true })

const profiles = computed(() => instances.value.filter((i) => i.kind !== 'server'))
const current = computed(() => profiles.value.find((i) => i.name === selected.value) ?? null)
const busy = computed(() => state.progress.active)

async function refresh() {
  instances.value = (await guard(api.listInstances)) ?? []
  if (selected.value && !current.value) selected.value = ''
}
async function loadMods() {
  mods.value = current.value?.loader === 'fabric' ? ((await guard(() => api.listMods(current.value!.name))) ?? []) : []
}
async function loadVersions() {
  if (versions.value.length) return
  const v = await guard(api.mcVersions)
  if (v) {
    versions.value = v.versions
    form.value.mc ||= v.latest
  }
}
watch(selected, (v) => {
  hits.value = []
  query.value = ''
  if (v) loadMods()
  else loadVersions()
})
onMounted(async () => {
  await refresh()
  if (selected.value) loadMods()
  else loadVersions()
  if (!form.value.name) form.value.name = profiles.value.length ? '' : 'Lava Optimized'
})

async function create() {
  const f = { ...form.value }
  const inst = await guard(() => api.createInstance(f.name, f.mc, f.fabric || f.preset, f.preset))
  if (inst) {
    toast('success', t('profile.ready', { name: inst.name }))
    state.profile = inst.name
    await refresh()
    selected.value = inst.name
    form.value.name = ''
  }
}
async function remove() {
  if (!current.value || !confirm(t('profile.confirmDelete', { name: current.value.name }))) return
  await guard(() => api.deleteInstance(current.value!.name))
  selected.value = ''
  await refresh()
}
async function toggle(m: ModFile) { await guard(() => api.toggleMod(current.value!.name, m.file)); loadMods() }
async function del(m: ModFile) { await guard(() => api.deleteMod(current.value!.name, m.file)); loadMods() }
async function search() {
  if (!current.value) return
  searching.value = true
  hits.value = (await guard(() => api.modrinthSearch(query.value, current.value!.mc)))?.hits ?? []
  searching.value = false
}
async function install(h: ModHit) {
  installing.value = h.project_id
  const r = await guard(() => api.modrinthInstall(current.value!.name, h.project_id))
  installing.value = ''
  if (r) { toast('success', t('profile.installed', { n: r.length })); loadMods() }
}
const size = (n: number) => (n > 1e6 ? (n / 1e6).toFixed(1) + ' MB' : Math.round(n / 1e3) + ' KB')
const num = (n: number) => (n > 1e6 ? (n / 1e6).toFixed(1) + 'M' : n > 1e3 ? Math.round(n / 1e3) + 'K' : String(n))
</script>

<template>
  <div class="modal-back" @click.self="emit('close')">
    <div class="mgr card">
      <div class="mgr-list">
        <div class="head"><h2>{{ t('profile.title') }}</h2></div>
        <div class="mgr-items">
          <button v-for="p in profiles" :key="p.name" class="pk-item" :class="{ on: p.name === selected }" @click="selected = p.name">
            <span style="flex: 1; overflow: hidden; text-overflow: ellipsis">{{ p.name }}</span>
            <span class="tag cool">{{ p.mc }}</span>
          </button>
        </div>
        <button class="pk-item add" style="padding: 12px 16px" :class="{ on: !selected }" @click="selected = ''">
          <PxIcon name="plus" :size="12" /> {{ t('profile.new') }}
        </button>
      </div>

      <div class="mgr-body">
        <div class="row-between">
          <h2 style="margin: 0">{{ current ? current.name : t('profile.newTitle') }}</h2>
          <button class="btn sm" @click="emit('close')">{{ t('common.close') }}</button>
        </div>

        <!-- new profile -->
        <form v-if="!current" class="stack" @submit.prevent="create">
          <div class="field"><label>{{ t('common.name') }}</label><input v-model="form.name" maxlength="40" placeholder="My profile" /></div>
          <div class="field">
            <label>{{ t('profile.mcVersion') }}</label>
            <select v-model="form.mc"><option v-for="v in versions" :key="v">{{ v }}</option></select>
            <div class="hint">{{ t('profile.versionHint') }}</div>
          </div>
          <div class="sect">
            <label class="check"><input v-model="form.preset" type="checkbox" /> {{ t('profile.preset') }}</label>
            <div class="small muted" style="margin: 2px 0 10px 26px">{{ t('profile.presetHint') }}</div>
            <label class="check"><input v-model="form.fabric" type="checkbox" :disabled="form.preset" /> {{ t('profile.fabric') }}</label>
          </div>
          <div><button class="btn primary" :disabled="busy || !form.name.trim() || !form.mc">{{ busy ? t('profile.installing') : t('common.create') }}</button></div>
        </form>

        <!-- existing profile -->
        <template v-else>
          <div class="row">
            <span class="tag cool">{{ current.mc }}</span>
            <span class="tag cool">{{ current.loader === 'fabric' ? `FABRIC ${current.loader_version}` : 'VANILLA' }}</span>
            <span v-if="current.kind === 'preset'" class="tag">LAVA PRESET</span>
            <span class="spacer"></span>
            <button class="btn sm" @click="api.openFolder('instances/' + current.name)">{{ t('profile.openFolder') }}</button>
            <button class="btn sm danger" :disabled="busy" @click="remove">{{ t('common.delete') }}</button>
          </div>

          <template v-if="current.loader === 'fabric'">
            <div class="sect">
              <h3>{{ t('profile.mods', { n: mods.length }) }}</h3>
              <div v-if="!mods.length" class="muted">{{ t('profile.noMods') }}</div>
              <div v-for="m in mods" :key="m.file" class="mod-row" :class="{ off: !m.enabled }">
                <input type="checkbox" :checked="m.enabled" :title="m.enabled ? t('profile.turnOff') : t('profile.turnOn')" @change="toggle(m)" />
                <span class="mod-name mono">{{ m.file.replace('.disabled', '') }}</span>
                <span class="small muted">{{ size(m.size) }}</span>
                <button class="btn sm danger ghost" @click="del(m)">{{ t('profile.remove') }}</button>
              </div>
            </div>
            <div class="sect">
              <h3>{{ t('profile.findMods') }}</h3>
              <form class="row" @submit.prevent="search">
                <input v-model="query" style="flex: 1; min-width: 160px" :placeholder="t('profile.searchPlaceholder', { mc: current.mc })" />
                <button class="btn" :disabled="searching">{{ t('profile.search') }}</button>
              </form>
              <div v-for="h in hits" :key="h.project_id" class="hit">
                <img v-if="h.icon_url" :src="h.icon_url" alt="" />
                <div v-else class="ph"></div>
                <div class="body">
                  <div class="t">{{ h.title }} <span class="small muted">{{ t('profile.byline', { author: h.author, n: num(h.downloads) }) }}</span></div>
                  <div class="small muted">{{ h.description }}</div>
                </div>
                <button class="btn sm primary" :disabled="busy || installing === h.project_id" @click="install(h)">{{ t('profile.install') }}</button>
              </div>
            </div>
          </template>
          <div v-else class="banner info">{{ t('profile.noLoader') }}</div>
        </template>
      </div>
    </div>
  </div>
</template>
