<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { api, type Instance } from '../api'
import { guard } from '../store'
import PxIcon from './PxIcon.vue'

defineProps<{ closable: boolean }>()
const emit = defineEmits<{ added: [Instance]; close: [] }>()

const address = ref('')
const busy = ref(false)
const input = ref<HTMLInputElement>()
onMounted(() => input.value?.focus())

async function add() {
  busy.value = true
  const inst = await guard(() => api.serverAdd(address.value))
  busy.value = false
  if (inst) {
    address.value = ''
    emit('added', inst)
  }
}
</script>

<template>
  <form class="connect card" @submit.prevent="add">
    <button v-if="closable" type="button" class="x" title="닫기" @click="emit('close')"><PxIcon name="close" :size="12" /></button>
    <h2>서버 연결</h2>
    <div class="field">
      <label>서버 주소 또는 manifest URL</label>
      <input ref="input" v-model="address" placeholder="play.myserver.com" />
      <div class="hint">서버의 <span class="mono">/launcher/manifest.json</span>을 읽어 버전, 모드, 아이콘, 배경을 가져옵니다.</div>
    </div>
    <div class="actions">
      <button class="btn primary" :disabled="busy || !address.trim()">{{ busy ? '확인 중...' : '연결' }}</button>
    </div>
  </form>
</template>
