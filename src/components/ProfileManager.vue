<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { api, type Instance, type ModFile, type ModHit } from '../api'
import { guard, state, toast } from '../store'
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
    toast('success', `"${inst.name}" 프로필이 준비됐습니다`)
    state.profile = inst.name
    await refresh()
    selected.value = inst.name
    form.value.name = ''
  }
}
async function remove() {
  if (!current.value || !confirm(`"${current.value.name}" 프로필과 모든 파일을 삭제할까요?`)) return
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
  if (r) { toast('success', `${r.length}개 파일 설치됨`); loadMods() }
}
const size = (n: number) => (n > 1e6 ? (n / 1e6).toFixed(1) + ' MB' : Math.round(n / 1e3) + ' KB')
const num = (n: number) => (n > 1e6 ? (n / 1e6).toFixed(1) + 'M' : n > 1e3 ? Math.round(n / 1e3) + 'K' : String(n))
</script>

<template>
  <div class="modal-back" @click.self="emit('close')">
    <div class="mgr card">
      <div class="mgr-list">
        <div class="head"><h2>프로필</h2></div>
        <div class="mgr-items">
          <button v-for="p in profiles" :key="p.name" class="pk-item" :class="{ on: p.name === selected }" @click="selected = p.name">
            <span style="flex: 1; overflow: hidden; text-overflow: ellipsis">{{ p.name }}</span>
            <span class="tag cool">{{ p.mc }}</span>
          </button>
        </div>
        <button class="pk-item add" style="padding: 12px 16px" :class="{ on: !selected }" @click="selected = ''">
          <PxIcon name="plus" :size="12" /> 새 프로필
        </button>
      </div>

      <div class="mgr-body">
        <div class="row-between">
          <h2 style="margin: 0">{{ current ? current.name : '새 프로필 만들기' }}</h2>
          <button class="btn sm" @click="emit('close')">닫기</button>
        </div>

        <!-- new profile -->
        <form v-if="!current" class="stack" @submit.prevent="create">
          <div class="field"><label>이름</label><input v-model="form.name" maxlength="40" placeholder="My profile" /></div>
          <div class="field">
            <label>마인크래프트 버전</label>
            <select v-model="form.mc"><option v-for="v in versions" :key="v">{{ v }}</option></select>
            <div class="hint">1.19 이상만 지원합니다.</div>
          </div>
          <div class="sect">
            <label class="check"><input v-model="form.preset" type="checkbox" /> Lava 프리셋 (Fabric API, Sodium, Iris, Lithium, FerriteCore)</label>
            <div class="small muted" style="margin: 2px 0 10px 26px">셰이더를 쓸 수 있는 최적화 구성을 자동으로 설치합니다.</div>
            <label class="check"><input v-model="form.fabric" type="checkbox" :disabled="form.preset" /> Fabric 로더 (모드를 쓰려면 필요)</label>
          </div>
          <div><button class="btn primary" :disabled="busy || !form.name.trim() || !form.mc">{{ busy ? '설치 중...' : '만들기' }}</button></div>
        </form>

        <!-- existing profile -->
        <template v-else>
          <div class="row">
            <span class="tag cool">{{ current.mc }}</span>
            <span class="tag cool">{{ current.loader === 'fabric' ? `FABRIC ${current.loader_version}` : 'VANILLA' }}</span>
            <span v-if="current.kind === 'preset'" class="tag">LAVA PRESET</span>
            <span class="spacer"></span>
            <button class="btn sm" @click="api.openFolder('instances/' + current.name)">폴더 열기</button>
            <button class="btn sm danger" :disabled="busy" @click="remove">삭제</button>
          </div>

          <template v-if="current.loader === 'fabric'">
            <div class="sect">
              <h3>설치된 모드 ({{ mods.length }})</h3>
              <div v-if="!mods.length" class="muted">설치된 모드가 없습니다.</div>
              <div v-for="m in mods" :key="m.file" class="mod-row" :class="{ off: !m.enabled }">
                <input type="checkbox" :checked="m.enabled" :title="m.enabled ? '끄기' : '켜기'" @change="toggle(m)" />
                <span class="mod-name mono">{{ m.file.replace('.disabled', '') }}</span>
                <span class="small muted">{{ size(m.size) }}</span>
                <button class="btn sm danger ghost" @click="del(m)">제거</button>
              </div>
            </div>
            <div class="sect">
              <h3>모드 찾기 (Modrinth)</h3>
              <form class="row" @submit.prevent="search">
                <input v-model="query" style="flex: 1; min-width: 160px" :placeholder="`${current.mc}용 모드 검색`" />
                <button class="btn" :disabled="searching">검색</button>
              </form>
              <div v-for="h in hits" :key="h.project_id" class="hit">
                <img v-if="h.icon_url" :src="h.icon_url" alt="" />
                <div v-else class="ph"></div>
                <div class="body">
                  <div class="t">{{ h.title }} <span class="small muted">by {{ h.author }} · {{ num(h.downloads) }} 다운로드</span></div>
                  <div class="small muted">{{ h.description }}</div>
                </div>
                <button class="btn sm primary" :disabled="busy || installing === h.project_id" @click="install(h)">설치</button>
              </div>
            </div>
          </template>
          <div v-else class="banner info">이 프로필은 모드 로더가 없습니다. 모드를 쓰려면 Fabric 프로필을 만드세요.</div>
        </template>
      </div>
    </div>
  </div>
</template>
