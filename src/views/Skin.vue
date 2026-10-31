<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { api, type SkinEntry } from '../api'
import { skinFront } from '../pixel'
import { background, guard, state, toast } from '../store'
import PxIcon from '../components/PxIcon.vue'
import SkinViewer from '../components/SkinViewer.vue'
import SkinAddModal from '../components/SkinAddModal.vue'
import { t } from '../i18n'

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
const sourceKey: Record<string, string> = {
  microsoft: 'skin.srcMicrosoft',
  local: 'skin.srcLocal',
  lookup: 'skin.srcLookup',
  none: 'skin.srcNone',
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
  if (!confirm(t('skin.confirmDelete', { name: e.name }))) return
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
              <h3 style="margin: 0">{{ preview ? t('skin.preview') : t('skin.current') }}</h3>
              <button v-if="preview" class="btn sm ghost" @click="preview = null">{{ t('skin.showCurrent') }}</button>
            </div>
            <div class="skin-stage"><SkinViewer :src="shown.src" :slim="shown.slim" /></div>
            <div class="small muted" style="padding: 6px 2px">
              <b style="color: #fff">{{ account?.name }}</b> · {{ shown.slim ? 'Slim' : 'Classic' }} ·
              {{ preview ? t('skin.notApplied') : sourceKey[current.source] ? t(sourceKey[current.source]) : '' }}
            </div>
            <button v-if="preview" class="btn primary" :disabled="applying" @click="apply(preview)">{{ t('skin.applyThis') }}</button>
            <div v-if="account?.kind !== 'microsoft'" class="small muted" style="margin-top: 8px">
              {{ t('skin.offlineNote') }}
            </div>
          </div>

          <div>
            <div class="row-between" style="margin-bottom: 10px">
              <h3 style="margin: 0">{{ t('skin.mine', { n: library.length }) }}</h3>
            </div>
            <div class="skin-lib">
              <div class="skin-card new" @click="adding = true">
                <PxIcon name="plus" :size="28" />
                <span>{{ t('skin.add') }}</span>
              </div>
              <div v-for="e in library" :key="e.id" class="skin-card" :class="{ on: preview === e.id }" @click="preview = e.id">
                <button class="del" :title="t('common.delete')" @click.stop="remove(e)"><PxIcon name="close" :size="10" /></button>
                <img :src="e.thumb" alt="" />
                <span class="n">{{ e.name }}</span>
                <span class="small muted">{{ e.variant === 'slim' ? 'Slim' : 'Classic' }}</span>
                <button class="btn sm primary" :disabled="applying" @click.stop="apply(e.id)">{{ t('skin.apply') }}</button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
    <SkinAddModal v-if="adding" @added="added" @close="adding = false" />
  </div>
</template>
