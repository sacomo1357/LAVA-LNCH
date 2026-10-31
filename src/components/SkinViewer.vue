<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import * as THREE from 'three'

const props = defineProps<{ src: string | null; slim: boolean }>()
const canvas = ref<HTMLCanvasElement>()

let renderer: THREE.WebGLRenderer
let scene: THREE.Scene
let camera: THREE.PerspectiveCamera
let model: THREE.Group
let limbs: { arm: THREE.Object3D[]; leg: THREE.Object3D[] } = { arm: [], leg: [] }
let tex: THREE.CanvasTexture | null = null
let raf = 0
let yaw = 0.6
let dragging = false
let lastX = 0

// UV origins on the 64x64 skin sheet (x, y)
const UV = {
  head: [0, 0], hat: [32, 0], body: [16, 16], jacket: [16, 32],
  rArm: [40, 16], rSleeve: [40, 32], lArm: [32, 48], lSleeve: [48, 48],
  rLeg: [0, 16], rPant: [0, 32], lLeg: [16, 48], lPant: [0, 48],
} as const

/** Box whose faces sample the skin sheet like Minecraft does. */
function skinBox(w: number, h: number, d: number, [u, v]: readonly [number, number], grow: number, map: THREE.Texture, overlay: boolean) {
  const g = new THREE.BoxGeometry(w + grow, h + grow, d + grow)
  // regions in BoxGeometry face order: +x, -x, +y, -y, +z, -z
  const regions: [number, number, number, number][] = [
    [u + d + w, v + d, d, h], // +x  (character's left)
    [u, v + d, d, h],         // -x  (character's right)
    [u + d, v, w, d],         // top
    [u + d + w, v, w, d],     // bottom
    [u + d, v + d, w, h],     // front
    [u + 2 * d + w, v + d, w, h], // back
  ]
  const uv = g.attributes.uv
  regions.forEach(([x, y, rw, rh], f) => {
    const u0 = x / 64, u1 = (x + rw) / 64, v0 = 1 - y / 64, v1 = 1 - (y + rh) / 64
    uv.setXY(f * 4 + 0, u0, v0)
    uv.setXY(f * 4 + 1, u1, v0)
    uv.setXY(f * 4 + 2, u0, v1)
    uv.setXY(f * 4 + 3, u1, v1)
  })
  const m = new THREE.MeshBasicMaterial({ map, transparent: overlay, alphaTest: overlay ? 0.5 : 0 })
  return new THREE.Mesh(g, m)
}

/** Limb whose bottom edge sits at height `y`; it swings from its top (shoulder / hip). */
function part(parent: THREE.Object3D, w: number, h: number, d: number, base: readonly [number, number], over: readonly [number, number], x: number, y: number, tx: THREE.Texture) {
  const pivot = new THREE.Group()
  pivot.position.set(x, y + h, 0)
  const a = skinBox(w, h, d, base, 0, tx, false)
  const b = skinBox(w, h, d, over, 0.6, tx, true)
  a.position.y = b.position.y = -h / 2
  pivot.add(a, b)
  parent.add(pivot)
  return pivot
}

function defaultSkin(): HTMLCanvasElement {
  const c = document.createElement('canvas')
  c.width = c.height = 64
  const g = c.getContext('2d')!
  const fill = (col: string, x: number, y: number, w: number, h: number) => { g.fillStyle = col; g.fillRect(x, y, w, h) }
  fill('#b58a67', 0, 0, 32, 16); fill('#4a3320', 8, 0, 16, 8); fill('#4a3320', 8, 8, 8, 2)
  fill('#ffffff', 10, 12, 2, 1); fill('#ffffff', 14, 12, 2, 1); fill('#4b3a9b', 11, 12, 1, 1); fill('#4b3a9b', 14, 12, 1, 1)
  fill('#1fa6a6', 16, 16, 24, 16); fill('#b58a67', 40, 16, 16, 16); fill('#3b3b9b', 0, 16, 16, 16)
  fill('#b58a67', 32, 48, 16, 16); fill('#3b3b9b', 16, 48, 16, 16)
  return c
}

function build(source: CanvasImageSource | null, srcH: number) {
  if (model) scene.remove(model)
  tex?.dispose()
  const sheet = document.createElement('canvas')
  sheet.width = sheet.height = 64
  const g = sheet.getContext('2d')!
  g.drawImage((source ?? defaultSkin()) as CanvasImageSource, 0, 0)
  tex = new THREE.CanvasTexture(sheet)
  tex.magFilter = THREE.NearestFilter
  tex.minFilter = THREE.NearestFilter
  tex.colorSpace = THREE.SRGBColorSpace
  const legacy = srcH === 32 // 64x32 skins have no separate left limbs
  const aw = props.slim ? 3 : 4

  model = new THREE.Group()
  limbs = { arm: [], leg: [] }
  const head = skinBox(8, 8, 8, UV.head, 0, tex, false)
  const hat = skinBox(8, 8, 8, UV.hat, 1, tex, true)
  head.position.y = hat.position.y = 28
  const body = skinBox(8, 12, 4, UV.body, 0, tex, false)
  const jacket = skinBox(8, 12, 4, UV.jacket, 0.6, tex, true)
  body.position.y = jacket.position.y = 18
  model.add(head, hat, body, jacket)
  limbs.arm.push(
    part(model, aw, 12, 4, UV.rArm, UV.rSleeve, -(4 + aw / 2), 12, tex),
    part(model, aw, 12, 4, legacy ? UV.rArm : UV.lArm, legacy ? UV.rSleeve : UV.lSleeve, 4 + aw / 2, 12, tex),
  )
  limbs.leg.push(
    part(model, 4, 12, 4, UV.rLeg, UV.rPant, -2, 0, tex),
    part(model, 4, 12, 4, legacy ? UV.rLeg : UV.lLeg, legacy ? UV.rPant : UV.lPant, 2, 0, tex),
  )
  scene.add(model)
}

let img: HTMLImageElement | null = null
function load() {
  if (!props.src) return build(null, 64)
  img = new Image()
  img.onload = () => build(img, img!.height)
  img.src = props.src
}

let size = { w: 0, h: 0 }
/** Move the camera back until the whole player (32 tall, ~20 wide with swinging arms) fits the frame. */
function fit(aspect: number) {
  const t = Math.tan(THREE.MathUtils.degToRad(camera.fov / 2))
  const dist = Math.max(19 / t, 12 / (t * aspect))
  camera.aspect = aspect
  camera.position.set(0, 18, dist)
  camera.lookAt(0, 16, 0)
  camera.updateProjectionMatrix()
}

function loop(t: number) {
  raf = requestAnimationFrame(loop)
  if (!dragging) yaw += 0.006
  model.rotation.y = yaw
  const s = Math.sin(t / 450) * 0.45
  limbs.arm[0].rotation.x = s; limbs.arm[1].rotation.x = -s
  limbs.leg[0].rotation.x = -s; limbs.leg[1].rotation.x = s
  const el = canvas.value!
  const w = el.clientWidth, h = el.clientHeight
  if (w && h && (w !== size.w || h !== size.h)) {
    size = { w, h }
    renderer.setSize(w, h, false)
    fit(w / h)
  }
  renderer.render(scene, camera)
}

onMounted(() => {
  renderer = new THREE.WebGLRenderer({ canvas: canvas.value!, alpha: true, antialias: false })
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2))
  scene = new THREE.Scene()
  camera = new THREE.PerspectiveCamera(30, 1, 1, 1000)
  fit(1)
  load()
  raf = requestAnimationFrame(loop)
})
onBeforeUnmount(() => {
  cancelAnimationFrame(raf)
  tex?.dispose()
  renderer.dispose()
})
watch(() => [props.src, props.slim], load)

const down = (e: PointerEvent) => { dragging = true; lastX = e.clientX; (e.target as HTMLElement).setPointerCapture(e.pointerId) }
const move = (e: PointerEvent) => { if (dragging) { yaw += (e.clientX - lastX) * 0.012; lastX = e.clientX } }
const up = () => { dragging = false }
</script>

<template>
  <canvas ref="canvas" @pointerdown="down" @pointermove="move" @pointerup="up" @pointercancel="up"></canvas>
</template>
