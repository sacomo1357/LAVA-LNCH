<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { api, type UpdateInfo } from '../api'
import { state, toast } from '../store'
import { t } from '../i18n'
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
      <h2>{{ t('update.title') }}</h2>
      <div class="ver">
        <span class="tag cool">v{{ info.current }}</span>
        <span class="arrow">→</span>
        <span class="tag">v{{ info.latest }}</span>
      </div>
      <pre v-if="info.notes" class="notes">{{ info.notes }}</pre>

      <template v-if="installing">
        <ProgressLine />
        <p class="small muted">{{ t('update.restartHint') }}</p>
      </template>
      <p v-else-if="left > 0" class="small muted">{{ t('update.countdown', { sec: left, mb }) }}</p>

      <div class="foot">
        <button class="btn" :disabled="installing" @click="later">{{ t('update.later') }}</button>
        <button class="btn primary" :disabled="installing" @click="install">{{ installing ? t('update.updating') : t('update.now') }}</button>
      </div>
    </div>
  </div>
</template>
