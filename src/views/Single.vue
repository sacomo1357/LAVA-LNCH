<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { api, type Instance } from '../api'
import { background, guard, play, state } from '../store'
import PxIcon from '../components/PxIcon.vue'
import ProgressLine from '../components/ProgressLine.vue'
import ProfileManager from '../components/ProfileManager.vue'

const instances = ref<Instance[]>([])
const menu = ref(false)
const manager = ref<{ open: boolean; create: boolean }>({ open: false, create: false })

const profiles = computed(() => instances.value.filter((i) => i.kind !== 'server'))
const current = computed(() => profiles.value.find((p) => p.name === state.profile) ?? profiles.value[0] ?? null)
const busy = computed(() => state.progress.active || state.playing)

async function refresh() {
  instances.value = (await guard(api.listInstances)) ?? []
  if (current.value) state.profile = current.value.name
}
onMounted(refresh)

function pick(p: Instance) {
  state.profile = p.name
  menu.value = false
}
function openManager(create: boolean) {
  menu.value = false
  manager.value = { open: true, create }
}
async function closeManager() {
  manager.value.open = false
  await refresh()
}
function launch() {
  if (current.value) play(() => api.launchInstance(current.value!.name))
  else openManager(true)
}
const loaderLabel = (p: Instance) => (p.loader === 'fabric' ? 'Fabric' : 'Vanilla')
</script>

<template>
  <div class="scene" :style="{ backgroundImage: `url(${background()})` }" @click="menu = false">
    <div class="hero">
      <h1>{{ current ? current.name.replace(/_/g, ' ') : 'LAVA SE' }}</h1>
      <p v-if="current">Minecraft {{ current.mc }} · {{ loaderLabel(current) }}<template v-if="current.kind === 'preset'"> · Lava 프리셋</template></p>
      <p v-else>프로필을 만들고 바로 플레이하세요.</p>
    </div>

    <div class="play-panel" @click.stop>
      <div class="play-row">
        <div class="picker">
          <button :disabled="busy" @click="menu = !menu">
            <div style="min-width: 0">
              <div class="small muted">프로필</div>
              <div class="pk-name">{{ current ? current.name : '프로필 없음' }}</div>
            </div>
            <span v-if="current" class="tag cool">{{ current.mc }}</span>
            <span class="caret">▲</span>
          </button>
          <div v-if="menu" class="pk-menu">
            <button v-for="p in profiles" :key="p.name" class="pk-item" :class="{ on: current?.name === p.name }" @click="pick(p)">
              <span style="flex: 1; overflow: hidden; text-overflow: ellipsis">{{ p.name }}</span>
              <span class="tag cool">{{ p.mc }}</span>
              <span v-if="p.loader === 'fabric'" class="tag cool">FABRIC</span>
            </button>
            <button class="pk-item add" @click="openManager(true)"><PxIcon name="plus" :size="12" /> 새 프로필 만들기</button>
          </div>
        </div>
        <button class="btn icon-btn" title="프로필 구성" :disabled="busy" @click="openManager(false)"><PxIcon name="gear" /></button>
        <button class="btn primary play-btn" :disabled="busy" @click="launch">
          <PxIcon name="play" :size="14" />{{ state.playing ? '실행 중' : current ? 'PLAY' : '만들기' }}
        </button>
      </div>
      <ProgressLine />
    </div>

    <ProfileManager v-if="manager.open" :start-new="manager.create || !profiles.length" :initial="current?.name ?? ''" @close="closeManager" />
  </div>
</template>
