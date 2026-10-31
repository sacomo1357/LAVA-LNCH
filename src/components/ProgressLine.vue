<script setup lang="ts">
import { computed } from 'vue'
import { state } from '../store'
import { t } from '../i18n'

const pct = computed(() => (state.progress.total ? Math.min(100, (state.progress.done / state.progress.total) * 100) : 0))
</script>

<template>
  <div v-if="state.progress.active" class="prog">
    <div class="meter"><div :style="{ width: pct + '%' }"></div></div>
    <span class="label small muted">{{ state.progress.label }} {{ state.progress.total ? `(${state.progress.done}/${state.progress.total})` : '' }}</span>
  </div>
  <div v-else-if="state.log.length" class="row-between small">
    <span :class="['pill', state.playing ? 'running' : 'offline']">{{ state.playing ? t('progress.playing') : t('progress.idle') }}</span>
    <a href="#" @click.prevent="state.showLog = true">{{ t('progress.showLog') }}</a>
  </div>
</template>
