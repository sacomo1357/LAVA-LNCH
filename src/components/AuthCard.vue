<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { api } from '../api'
import { guard, state } from '../store'

const mode = ref<'offline' | 'microsoft'>('microsoft')
const name = ref('')
const clientId = ref(state.settings?.ms_client_id ?? '')
const waiting = ref(false)
const builtin = ref(false) // the LAVA LNCH client ID ships with the launcher
onMounted(async () => { builtin.value = await api.builtinClientId().catch(() => false) })

async function offline() {
  const s = await guard(() => api.authOffline(name.value))
  if (s) state.settings = s
}

async function microsoft() {
  if (!state.settings || waiting.value) return
  waiting.value = true
  try {
    if (clientId.value.trim() !== state.settings.ms_client_id) {
      state.settings = await api.saveSettings({ ...state.settings, ms_client_id: clientId.value.trim() })
    }
    const s = await guard(api.authMsLogin)
    if (s) state.settings = s
  } finally {
    waiting.value = false
  }
}

function cancel() {
  api.authMsCancel().catch(() => {})
}
</script>

<template>
  <div class="auth-wrap">
    <form class="card auth-card" @submit.prevent="mode === 'offline' ? offline() : microsoft()">
      <div class="brand"><img src="/favicon.svg" alt="" /><span>LAVA SE</span></div>
      <h2>로그인</h2>
      <div class="seg">
        <button type="button" :class="{ on: mode === 'microsoft' }" :disabled="waiting" @click="mode = 'microsoft'">Microsoft</button>
        <button type="button" :class="{ on: mode === 'offline' }" :disabled="waiting" @click="mode = 'offline'">오프라인</button>
      </div>

      <template v-if="mode === 'offline'">
        <div class="field">
          <label>플레이어 이름</label>
          <input v-model="name" maxlength="16" placeholder="Steve" />
          <div class="hint">오프라인 계정은 정품 인증 서버에 접속할 수 없습니다.</div>
        </div>
        <button class="btn primary" style="width: calc(100% - 6px)">계속</button>
      </template>

      <template v-else-if="!waiting">
        <div v-if="!builtin" class="field">
          <label>Azure 앱 (client) ID</label>
          <input v-model="clientId" placeholder="00000000-0000-0000-0000-000000000000" />
          <div class="hint">Azure 앱 "LAVA LNCH"의 애플리케이션(클라이언트) ID입니다. 리디렉션 URI는 http://localhost:9876 이어야 합니다.</div>
        </div>
        <p v-else class="muted">브라우저에서 Microsoft 계정으로 로그인합니다.</p>
        <button class="btn primary" :disabled="!builtin && !clientId.trim()" style="width: calc(100% - 6px)">Microsoft로 로그인</button>
      </template>

      <template v-else>
        <div class="code-box">
          <div class="small muted">브라우저에서 Microsoft 로그인을 마치세요</div>
          <div style="margin-top: 8px"><span class="pill starting">대기 중</span></div>
        </div>
        <p class="small muted">로그인이 끝나면 자동으로 런처로 돌아옵니다. 브라우저가 열리지 않았다면 다시 시도하세요.</p>
        <button type="button" class="btn" style="width: calc(100% - 6px)" @click="cancel">취소</button>
      </template>
    </form>
  </div>
</template>
