<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { api, type SkinEntry } from '../api'
import { skinFront } from '../pixel'
import { background, guard, state, toast } from '../store'
import PxIcon from '../components/PxIcon.vue'
import SkinViewer from '../components/SkinViewer.vue'
import SkinAddModal from '../components/SkinAddModal.vue'

const account = computed(() => state.settings?.account)
const current = ref<{ src: string | null; slim: boolean; source: string }>({ src: null, slim: false, source: '' })
const library = ref<(SkinEntry & { thumb: string })[]>([])
const preview = ref<string | null>(null) // library id shown in the viewer, null = applied skin
const adding = ref(false)
const applying = ref(false)

const shown = computed(() => {
  const e = library.value.find((s) => s.id === preview.value)
  return e ? { src: 'data:image/png;base64,' + e.png_b64, slim: e.variant === 'slim' } : current.value
})
const sourceLabel: Record<string, string> = {
  microsoft: 'Microsoft 계정에 적용된 스킨',
  local: '이 런처에 저장된 스킨',
  lookup: '같은 이름의 정품 계정 스킨',
  none: '스킨이 없어 기본 모습을 보여줍니다',
}

async function loadCurrent() {
  const s = await guard(api.skinGet)
  if (s) current.value = { src: s.png_b64 ? 'data:image/png;base64,' + s.png_b64 : null, slim: s.variant === 'slim', source: s.source }
}
async function loadLibrary() {
  const list = (await guard(api.skinList)) ?? []
  library.value = await Promise.all(list.map(async (e) => ({ ...e, thumb: await skinFront('data:image/png;base64,' + e.png_b64, e.variant === 'slim') })))
}
onMounted(() => { loadCurrent(); loadLibrary() })

async function added(e: SkinEntry) {
  adding.value = false
  await loadLibrary()
  preview.value = e.id
}
async function apply(id: string) {
  applying.value = true
  const msg = await guard(() => api.skinApply(id))
  applying.value = false
  if (msg) {
    toast('success', msg)
    preview.value = null
    await loadCurrent()
  }
}
async function remove(e: SkinEntry) {
  if (!confirm(`"${e.name}" 스킨을 목록에서 지울까요?`)) return
  await guard(() => api.skinDelete(e.id))
  if (preview.value === e.id) preview.value = null
  loadLibrary()
}
</script>

<template>
  <div class="scene" :style="{ backgroundImage: `url(${background()})` }">
    <div class="framed">
      <div class="page">
        <div class="skin-grid">
          <div class="skin-cur">
            <div class="row-between">
              <h3 style="margin: 0">{{ preview ? '미리보기' : '현재 스킨' }}</h3>
              <button v-if="preview" class="btn sm ghost" @click="preview = null">현재 스킨 보기</button>
            </div>
            <div class="skin-stage"><SkinViewer :src="shown.src" :slim="shown.slim" /></div>
            <div class="small muted" style="padding: 6px 2px">
              <b style="color: #fff">{{ account?.name }}</b> · {{ shown.slim ? 'Slim' : 'Classic' }} ·
              {{ preview ? '적용 전' : sourceLabel[current.source] ?? '' }}
            </div>
            <button v-if="preview" class="btn primary" :disabled="applying" @click="apply(preview)">이 스킨 적용하기</button>
            <div v-if="account?.kind !== 'microsoft'" class="small muted" style="margin-top: 8px">
              오프라인 계정은 런처에만 저장됩니다. 게임 안 스킨을 바꾸려면 Microsoft로 로그인하세요.
            </div>
          </div>

          <div>
            <div class="row-between" style="margin-bottom: 10px">
              <h3 style="margin: 0">내 스킨 ({{ library.length }})</h3>
            </div>
            <div class="skin-lib">
              <div class="skin-card new" @click="adding = true">
                <PxIcon name="plus" :size="28" />
                <span>스킨 추가</span>
              </div>
              <div v-for="e in library" :key="e.id" class="skin-card" :class="{ on: preview === e.id }" @click="preview = e.id">
                <button class="del" title="삭제" @click.stop="remove(e)"><PxIcon name="close" :size="10" /></button>
                <img :src="e.thumb" alt="" />
                <span class="n">{{ e.name }}</span>
                <span class="small muted">{{ e.variant === 'slim' ? 'Slim' : 'Classic' }}</span>
                <button class="btn sm primary" :disabled="applying" @click.stop="apply(e.id)">적용</button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
    <SkinAddModal v-if="adding" @added="added" @close="adding = false" />
  </div>
</template>
