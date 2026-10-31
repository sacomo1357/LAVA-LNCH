<script setup lang="ts">
import { ref } from 'vue'
import { api, type SkinEntry } from '../api'
import { detectSlim } from '../pixel'
import { guard, readDataUrl, toast } from '../store'
import SkinViewer from './SkinViewer.vue'

const emit = defineEmits<{ added: [SkinEntry]; close: [] }>()

const name = ref('')
const src = ref<string | null>(null)
const fileName = ref('')
const slim = ref(false)
const drag = ref(false)
const saving = ref(false)
const picker = ref<HTMLInputElement>()

async function take(file?: File) {
  if (!file) return
  if (file.type !== 'image/png') return toast('error', '스킨은 PNG 파일이어야 합니다')
  const url = await readDataUrl(file)
  const img = new Image()
  img.onload = () => {
    if (img.width !== 64 || (img.height !== 64 && img.height !== 32)) return toast('error', '스킨 크기는 64x64 (또는 64x32)여야 합니다')
    src.value = url
    fileName.value = file.name
    slim.value = detectSlim(img)
    if (!name.value.trim()) name.value = file.name.replace(/\.png$/i, '').slice(0, 32)
  }
  img.src = url
}

async function save() {
  if (!src.value) return
  saving.value = true
  const e = await guard(() => api.skinAdd(name.value, src.value!.split(',')[1], slim.value ? 'slim' : 'classic'))
  saving.value = false
  if (e) emit('added', e)
}
</script>

<template>
  <div class="modal-back" @click.self="emit('close')">
    <form class="skin-modal card" @submit.prevent="save">
      <div class="preview"><SkinViewer :src="src" :slim="slim" /></div>
      <div class="form">
        <h2>스킨 추가</h2>
        <div class="field">
          <label>스킨 이름</label>
          <input v-model="name" maxlength="32" placeholder="내 스킨" />
        </div>
        <div class="field">
          <label>스킨 파일 (PNG, 64x64)</label>
          <div class="dropzone" :class="{ drag }" @click="picker?.click()" @dragover.prevent="drag = true" @dragleave="drag = false"
            @drop.prevent="drag = false; take($event.dataTransfer?.files[0])">
            {{ fileName || 'PNG 파일을 끌어다 놓거나 클릭해서 고르세요' }}
          </div>
          <input ref="picker" type="file" accept="image/png" hidden @change="take(($event.target as HTMLInputElement).files?.[0])" />
        </div>
        <div class="field">
          <label>모델 <span class="small">(파일에서 자동 감지)</span></label>
          <div class="seg" style="margin: 0">
            <button type="button" :class="{ on: !slim }" @click="slim = false">Classic (Steve)</button>
            <button type="button" :class="{ on: slim }" @click="slim = true">Slim (Alex)</button>
          </div>
        </div>
        <div class="foot">
          <button type="button" class="btn" @click="emit('close')">취소</button>
          <button class="btn primary" :disabled="!src || !name.trim() || saving">확인</button>
        </div>
      </div>
    </form>
  </div>
</template>
