<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { api, type UpdateInfo } from './api'
import { guard, initEvents, loadSettings, state, type Tab } from './store'
import AuthCard from './components/AuthCard.vue'
import UpdateDialog from './components/UpdateDialog.vue'
import PxIcon from './components/PxIcon.vue'
import Single from './views/Single.vue'
import Multi from './views/Multi.vue'
import Skin from './views/Skin.vue'
import Setting from './views/Setting.vue'

const tabs: { id: Tab; label: string }[] = [
  { id: 'single', label: 'Single' },
  { id: 'multi', label: 'Multi' },
  { id: 'skin', label: 'Skin' },
  { id: 'setting', label: 'Setting' },
]
const account = computed(() => state.settings?.account ?? null)

const update = ref<UpdateInfo | null>(null)

onMounted(async () => {
  await initEvents()
  await guard(loadSettings)
  if (state.settings?.auto_update) {
    // quiet on failure: being offline should not nag at every start
    const u = await api.updateCheck().catch(() => null)
    if (u?.available && u.can_install) update.value = u
  }
})
</script>

<template>
  <div v-if="!state.settings" class="auth-wrap muted">Loading...</div>
  <AuthCard v-else-if="!account" />
  <div v-else class="app" :class="{ collapsed: state.collapsed }">
    <aside class="side">
      <div class="side-brand">
        <img src="/favicon.svg" alt="" />
        <span class="side-label">LAVA SE</span>
      </div>
      <nav class="side-nav">
        <button v-for="t in tabs" :key="t.id" class="side-item" :class="{ active: state.tab === t.id }" :title="t.label" @click="state.tab = t.id">
          <PxIcon :name="t.id" />
          <span class="side-label">{{ t.label }}</span>
        </button>
      </nav>
      <div class="side-foot">
        <button class="side-item who" :title="account.name" @click="state.tab = 'setting'">
          <span class="avatar">{{ account.name.slice(0, 1).toUpperCase() }}</span>
          <span class="side-label">
            <b>{{ account.name }}</b><br />
            <span class="small muted">{{ account.kind === 'microsoft' ? 'Microsoft' : '오프라인' }}</span>
          </span>
        </button>
        <button class="side-item toggle" :title="state.collapsed ? '펼치기' : '접기'" @click="state.collapsed = !state.collapsed">
          <span class="chev">{{ state.collapsed ? '»' : '«' }}</span>
          <span class="side-label">접기</span>
        </button>
      </div>
    </aside>

    <main class="stage">
      <Single v-if="state.tab === 'single'" />
      <Multi v-else-if="state.tab === 'multi'" />
      <Skin v-else-if="state.tab === 'skin'" />
      <Setting v-else />
    </main>

    <div v-if="state.showLog" class="modal-back" @click.self="state.showLog = false">
      <div class="modal card wide" style="max-width: 1000px">
        <div class="card-head"><h2>게임 로그</h2><span class="spacer"></span><button class="btn sm" @click="state.showLog = false">닫기</button></div>
        <div class="log">{{ state.log.join('\n') }}</div>
      </div>
    </div>
  </div>
  <UpdateDialog v-if="update" :info="update" @close="update = null" />
  <div id="toasts">
    <div v-for="t in state.toasts" :key="t.id" :class="['toast', t.kind === 'warn' ? '' : t.kind]">{{ t.msg }}</div>
  </div>
</template>
