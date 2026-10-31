<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { api, type UpdateInfo } from './api'
import { guard, initEvents, loadSettings, state, type Tab } from './store'
import { t } from './i18n'
import AuthCard from './components/AuthCard.vue'
import UpdateDialog from './components/UpdateDialog.vue'
import PxIcon from './components/PxIcon.vue'
import Single from './views/Single.vue'
import Multi from './views/Multi.vue'
import Skin from './views/Skin.vue'
import Setting from './views/Setting.vue'

const tabs: Tab[] = ['single', 'multi', 'skin', 'setting']
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
  <div v-if="!state.settings" class="auth-wrap muted">{{ t('app.loading') }}</div>
  <AuthCard v-else-if="!account" />
  <div v-else class="app" :class="{ collapsed: state.collapsed }">
    <aside class="side">
      <div class="side-brand">
        <img src="/favicon.svg" alt="" />
        <span class="side-label">LAVA LNCH</span>
      </div>
      <nav class="side-nav">
        <button v-for="id in tabs" :key="id" class="side-item" :class="{ active: state.tab === id }" :title="t('tab.' + id)" @click="state.tab = id">
          <PxIcon :name="id" />
          <span class="side-label">{{ t('tab.' + id) }}</span>
        </button>
      </nav>
      <div class="side-foot">
        <button class="side-item who" :title="account.name" @click="state.tab = 'setting'">
          <span class="avatar">{{ account.name.slice(0, 1).toUpperCase() }}</span>
          <span class="side-label">
            <b>{{ account.name }}</b><br />
            <span class="small muted">{{ account.kind === 'microsoft' ? 'Microsoft' : t('common.offline') }}</span>
          </span>
        </button>
        <button class="side-item toggle" :title="state.collapsed ? t('app.expand') : t('app.collapse')" @click="state.collapsed = !state.collapsed">
          <span class="chev">{{ state.collapsed ? '»' : '«' }}</span>
          <span class="side-label">{{ t('app.collapse') }}</span>
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
        <div class="card-head"><h2>{{ t('app.gameLog') }}</h2><span class="spacer"></span><button class="btn sm" @click="state.showLog = false">{{ t('common.close') }}</button></div>
        <div class="log">{{ state.log.join('\n') }}</div>
      </div>
    </div>
  </div>
  <UpdateDialog v-if="update" :info="update" @close="update = null" />
  <div id="toasts">
    <div v-for="t in state.toasts" :key="t.id" :class="['toast', t.kind === 'warn' ? '' : t.kind]">{{ t.msg }}</div>
  </div>
</template>
