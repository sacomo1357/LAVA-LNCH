<script setup lang="ts">
import { computed } from 'vue'
import { ICONS } from '../pixel'

const props = defineProps<{ name: string; size?: number }>()
const rects = computed(() => {
  const out: { x: number; y: number; w: number }[] = []
  ;(ICONS[props.name] ?? []).forEach((row, y) => {
    // merge horizontal runs into one rect each
    let x = 0
    while (x < row.length) {
      if (row[x] !== 'X') { x++; continue }
      const s = x
      while (row[x] === 'X') x++
      out.push({ x: s, y, w: x - s })
    }
  })
  return out
})
</script>

<template>
  <svg class="px-icon" viewBox="0 0 10 10" :width="size ?? 18" :height="size ?? 18" shape-rendering="crispEdges" aria-hidden="true">
    <rect v-for="(r, i) in rects" :key="i" :x="r.x" :y="r.y" :width="r.w" height="1" fill="currentColor" />
  </svg>
</template>
