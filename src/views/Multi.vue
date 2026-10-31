<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { api, type Instance } from '../api'
import { background, guard, play, state } from '../store'
import { creeperIcon } from '../pixel'
import { t } from '../i18n'
import ConnectCard from '../components/ConnectCard.vue'
import PxIcon from '../components/PxIcon.vue'
import ProgressLine from '../components/ProgressLine.vue'

const servers = ref<Instance[]>([])
const loaded = ref(false)
const adding = ref(false)
const failedIcons = ref(new Set<string>())

const current = computed(() => servers.value.find((s) => s.name === state.server) ?? servers.value[0] ?? null)
const busy = computed(() => state.progress.active || state.playing)

async function refresh() {
  servers.value = ((await guard(api.listInstances)) ?? []).filter((i) => i.kind === 'server')
  loaded.value = true
}
onMounted(refresh)

/** The server's own icon, or a creeper face coloured from its name. */
const icon = (s: Instance) => (s.icon && !failedIcons.value.has(s.name) ? s.icon : creeperIcon(s.name))
const bg = (s: Instance | null) => (s?.background ? `url(${JSON.stringify(s.background)}), url(${background()})` : `url(${background()})`)

async function added(i: Instance) {
  adding.value = false
  await refresh()
  state.server = i.name
}
function connect() {
  if (current.value) play(() => api.multiPlay(current.value!.address || current.value!.server))
}
async function remove() {
  const s = current.value
  if (!s || !confirm(t('multi.confirmDelete', { name: s.title || s.server }))) return
  await guard(() => api.deleteInstance(s.name))
  await refresh()
}
</script>

<template>
  <div v-if="loaded && !servers.length" class="scene" :style="{ backgroundImage: `url(${background()})` }">
    <ConnectCard :closable="false" @added="added" />
  </div>

  <div v-else-if="current" class="multi">
    <div class="srv-list">
      <div class="head"><h2 style="margin: 0">{{ t('multi.servers') }}</h2></div>
      <div class="srv-items">
        <button v-for="s in servers" :key="s.name" class="srv" :class="{ on: s.name === current.name }" @click="state.server = s.name">
          <img :src="icon(s)" alt="" @error="failedIcons.add(s.name)" />
          <div>
            <div class="t">{{ s.title || s.server }}</div>
            <div class="a">{{ s.server }}</div>
          </div>
        </button>
        <button class="btn srv-add" @click="adding = true"><PxIcon name="plus" :size="12" /> {{ t('multi.addServer') }}</button>
      </div>
    </div>

    <div class="srv-scene">
      <div class="scene" :style="{ backgroundImage: bg(current) }">
        <div class="hero">
          <div class="srv-head">
            <img :src="icon(current)" alt="" @error="failedIcons.add(current.name)" />
            <div>
              <h1>{{ current.title || current.server }}</h1>
              <p class="mono small">{{ current.server }}</p>
            </div>
          </div>
          <p v-if="current.description" style="margin-top: 12px">{{ current.description }}</p>
        </div>

        <div class="play-panel">
          <div class="play-row">
            <div style="flex: 1; min-width: 0; padding: 4px 2px">
              <div class="small muted">{{ t('multi.server') }}</div>
              <div class="pk-name" style="font-weight: 700; color: #fff; overflow: hidden; text-overflow: ellipsis; white-space: nowrap">{{ current.title || current.server }}</div>
              <div>
                <span class="tag cool">{{ current.mc }}</span>
                <span v-if="current.loader === 'fabric'" class="tag cool">FABRIC</span>
              </div>
            </div>
            <button class="btn icon-btn danger" :title="t('multi.deleteServer')" :disabled="busy" @click="remove"><PxIcon name="close" :size="12" /></button>
            <button class="btn primary play-btn" :disabled="busy" @click="connect"><PxIcon name="play" :size="14" />{{ state.playing ? t('common.running') : t('common.play') }}</button>
          </div>
          <ProgressLine />
        </div>
      </div>
    </div>

    <div v-if="adding" class="modal-back" @click.self="adding = false">
      <ConnectCard :closable="true" style="position: relative; left: auto; top: auto; transform: none" @added="added" @close="adding = false" />
    </div>
  </div>
</template>
