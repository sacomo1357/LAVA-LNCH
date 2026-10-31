<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { api, type UpdateInfo } from '../api'
import { state, toast } from '../store'
import ProgressLine from './ProgressLine.vue'

const props = defineProps<{ info: UpdateInfo }>()
const emit = defineEmits<{ close: [] }>()

const COUNTDOWN = 5
const left = ref(COUNTDOWN)
const installing = ref(false)
let timer = 0

const mb = computed(() => (props.info.size / 1e6).toFixed(1))

async function install() {
  clearInterval(timer)
  if (installing.value) return
  installing.value = true
  try {
    await api.updateInstall() // on success the launcher closes and the installer takes over
  } catch (e) {
    installing.value = false
    toast('error', String(e))
  }
}

onMounted(() => {
  // install by itself unless the user steps in; never in the middle of a game
  if (state.playing) left.value = -1
  else timer = window.setInterval(() => { if (--left.value <= 0) install() }, 1000)
})
onBeforeUnmount(() => clearInterval(timer))

function later() {
  clearInterval(timer)
  emit('close')
}
</script>

<template>
  <div class="modal-back">
    <div class="modal card update">
      <h2>새 버전이 있습니다</h2>
      <div class="ver">
        <span class="tag cool">v{{ info.current }}</span>
        <span class="arrow">→</span>
        <span class="tag">v{{ info.latest }}</span>
      </div>
      <pre v-if="info.notes" class="notes">{{ info.notes }}</pre>

      <template v-if="installing">
        <ProgressLine />
        <p class="small muted">다운로드가 끝나면 런처가 닫히고 설치 후 다시 열립니다.</p>
      </template>
      <p v-else-if="left > 0" class="small muted">{{ left }}초 뒤 자동으로 업데이트합니다 ({{ mb }} MB).</p>

      <div class="foot">
        <button class="btn" :disabled="installing" @click="later">나중에</button>
        <button class="btn primary" :disabled="installing" @click="install">{{ installing ? '업데이트 중...' : '지금 업데이트' }}</button>
      </div>
    </div>
  </div>
</template>
