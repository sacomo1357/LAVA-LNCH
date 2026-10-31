<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { api, type Instance } from '../api'
import { guard } from '../store'
import { t } from '../i18n'
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
    <button v-if="closable" type="button" class="x" :title="t('common.close')" @click="emit('close')"><PxIcon name="close" :size="12" /></button>
    <h2>{{ t('connect.title') }}</h2>
    <div class="field">
      <label>{{ t('connect.address') }}</label>
      <input ref="input" v-model="address" placeholder="play.myserver.com" />
      <div class="hint" v-html="t('connect.hint', { path: '<span class=&quot;mono&quot;>/launcher/manifest.json</span>' })"></div>
    </div>
    <div class="actions">
      <button class="btn primary" :disabled="busy || !address.trim()">{{ busy ? t('common.checking') : t('connect.connect') }}</button>
    </div>
  </form>
</template>
