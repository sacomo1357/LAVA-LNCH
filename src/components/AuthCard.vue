<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { api } from '../api'
import { guard, state } from '../store'
import { t } from '../i18n'

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
      <div class="brand"><img src="/favicon.svg" alt="" /><span>LAVA LNCH</span></div>
      <h2>{{ t('auth.title') }}</h2>
      <div class="seg">
        <button type="button" :class="{ on: mode === 'microsoft' }" :disabled="waiting" @click="mode = 'microsoft'">Microsoft</button>
        <button type="button" :class="{ on: mode === 'offline' }" :disabled="waiting" @click="mode = 'offline'">{{ t('common.offline') }}</button>
      </div>

      <template v-if="mode === 'offline'">
        <div class="field">
          <label>{{ t('auth.playerName') }}</label>
          <input v-model="name" maxlength="16" placeholder="Steve" />
          <div class="hint">{{ t('auth.offlineHint') }}</div>
        </div>
        <button class="btn primary" style="width: calc(100% - 6px)">{{ t('auth.continue') }}</button>
      </template>

      <template v-else-if="!waiting">
        <div v-if="!builtin" class="field">
          <label>{{ t('auth.clientId') }}</label>
          <input v-model="clientId" placeholder="00000000-0000-0000-0000-000000000000" />
          <div class="hint">{{ t('auth.clientIdHint') }}</div>
        </div>
        <p v-else class="muted">{{ t('auth.browserLogin') }}</p>
        <button class="btn primary" :disabled="!builtin && !clientId.trim()" style="width: calc(100% - 6px)">{{ t('auth.msLogin') }}</button>
      </template>

      <template v-else>
        <div class="code-box">
          <div class="small muted">{{ t('auth.finishInBrowser') }}</div>
          <div style="margin-top: 8px"><span class="pill starting">{{ t('auth.waiting') }}</span></div>
        </div>
        <p class="small muted">{{ t('auth.waitingHint') }}</p>
        <button type="button" class="btn" style="width: calc(100% - 6px)" @click="cancel">{{ t('common.cancel') }}</button>
      </template>
    </form>
  </div>
</template>
