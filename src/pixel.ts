// Small pixel-art generators: nav icons, creeper server icons, the default background, skin thumbnails.

export const ICONS: Record<string, string[]> = {
  single: [
    '...XXXX...',
    '...XXXX...',
    '...XXXX...',
    '..........',
    '.XXXXXXXX.',
    '.X.XXXX.X.',
    '.X.XXXX.X.',
    '...X..X...',
    '...X..X...',
    '...X..X...',
  ],
  multi: [
    'XXXXXXXXXX',
    'X........X',
    'X.XX...X.X',
    'XXXXXXXXXX',
    'X........X',
    'X.XX...X.X',
    'XXXXXXXXXX',
    '....XX....',
    '..XXXXXX..',
    '..........',
  ],
  skin: [
    '..XX..XX..',
    '.XXXXXXXX.',
    'XXXXXXXXXX',
    'XX.XXXX.XX',
    '...XXXX...',
    '...XXXX...',
    '...XXXX...',
    '...XXXX...',
    '...XXXX...',
    '..........',
  ],
  setting: [
    '....XX....',
    '.X.XXXX.X.',
    '..XXXXXX..',
    '.XXX..XXX.',
    'XXX....XXX',
    'XXX....XXX',
    '.XXX..XXX.',
    '..XXXXXX..',
    '.X.XXXX.X.',
    '....XX....',
  ],
  play: [
    '..X.......',
    '..XX......',
    '..XXX.....',
    '..XXXX....',
    '..XXXXX...',
    '..XXXXX...',
    '..XXXX....',
    '..XXX.....',
    '..XX......',
    '..X.......',
  ],
  gear: [
    '....XX....',
    '.X.XXXX.X.',
    '..XXXXXX..',
    '.XXX..XXX.',
    'XXX....XXX',
    'XXX....XXX',
    '.XXX..XXX.',
    '..XXXXXX..',
    '.X.XXXX.X.',
    '....XX....',
  ],
  plus: [
    '..........',
    '....XX....',
    '....XX....',
    '....XX....',
    '.XXXXXXXX.',
    '.XXXXXXXX.',
    '....XX....',
    '....XX....',
    '....XX....',
    '..........',
  ],
  close: [
    'XX......XX',
    'XXX....XXX',
    '.XXX..XXX.',
    '..XXXXXX..',
    '...XXXX...',
    '...XXXX...',
    '..XXXXXX..',
    '.XXX..XXX.',
    'XXX....XXX',
    'XX......XX',
  ],
}

/** Cheap deterministic PRNG so the same seed always draws the same picture. */
function rng(seed: number) {
  let s = seed >>> 0 || 1
  return () => ((s = (s * 1664525 + 1013904223) >>> 0) / 4294967296)
}

export function hash(str: string): number {
  let h = 2166136261
  for (const c of str) h = Math.imul(h ^ c.charCodeAt(0), 16777619)
  return h >>> 0
}

const CREEPER = [
  '........',
  '........',
  '.XX..XX.',
  '.XX..XX.',
  '...XX...',
  '..XXXX..',
  '..XXXX..',
  '..X..X..',
]

/** Creeper face in a colour picked from the seed (stable per server). */
export function creeperIcon(seed: string): string {
  const r = rng(hash(seed))
  const hue = Math.floor(r() * 360)
  const c = document.createElement('canvas')
  c.width = c.height = 8
  const g = c.getContext('2d')!
  for (let y = 0; y < 8; y++)
    for (let x = 0; x < 8; x++) {
      const face = CREEPER[y][x] === 'X'
      const l = face ? 8 + r() * 6 : 34 + r() * 18
      g.fillStyle = `hsl(${hue} ${face ? 30 : 55}% ${l}%)`
      g.fillRect(x, y, 1, 1)
    }
  return c.toDataURL()
}

let defaultBg = ''
/** Pixel-art lava landscape used when no custom background is set. */
export function defaultBackground(): string {
  if (defaultBg) return defaultBg
  const W = 240, H = 135
  const r = rng(7)
  const c = document.createElement('canvas')
  c.width = W
  c.height = H
  const g = c.getContext('2d')!
  // sky in hard bands
  const sky = ['#120c14', '#170e17', '#1d1018', '#241218', '#2d1517', '#381815', '#461c13']
  sky.forEach((col, i) => { g.fillStyle = col; g.fillRect(0, Math.floor((i * H * 0.62) / sky.length), W, Math.ceil((H * 0.62) / sky.length) + 1) })
  g.fillStyle = '#4f2012'
  g.fillRect(0, Math.floor(H * 0.62), W, H)
  for (let i = 0; i < 70; i++) {
    g.fillStyle = r() > 0.8 ? '#ffd9a8' : '#8a6f7a'
    g.fillRect(Math.floor(r() * W), Math.floor(r() * H * 0.45), 1, 1)
  }
  // mountain ridges, back to front
  const ridge = (base: number, amp: number, col: string, step: number) => {
    g.fillStyle = col
    let y = base
    for (let x = 0; x < W; x += step) {
      y = Math.max(base - amp, Math.min(base + amp * 0.4, y + (r() - 0.5) * amp * 0.5))
      g.fillRect(x, Math.floor(y), step, H)
    }
  }
  ridge(H * 0.5, 22, '#2a1720', 3)
  ridge(H * 0.62, 18, '#1f1219', 2)
  ridge(H * 0.74, 12, '#160d12', 2)
  // lava lake with glowing pixels
  const lake = Math.floor(H * 0.84)
  g.fillStyle = '#b8400f'
  g.fillRect(0, lake, W, H - lake)
  for (let i = 0; i < 520; i++) {
    const v = r()
    g.fillStyle = v > 0.92 ? '#fff1c4' : v > 0.7 ? '#ffb03a' : v > 0.35 ? '#ff6a2b' : '#d9531a'
    g.fillRect(Math.floor(r() * W), lake + Math.floor(r() * (H - lake)), 1 + Math.floor(r() * 2), 1)
  }
  // glow above the lake
  for (let y = lake - 14; y < lake; y++) {
    g.fillStyle = `rgba(255, 106, 43, ${(0.22 * (y - lake + 14)) / 14})`
    g.fillRect(0, y, W, 1)
  }
  defaultBg = c.toDataURL()
  return defaultBg
}

/** Flat front view of a skin (head, body, arms, legs with overlays) at 16x32. */
export function skinFront(src: string, slim: boolean): Promise<string> {
  return new Promise((resolve) => {
    const img = new Image()
    img.onload = () => {
      const c = document.createElement('canvas')
      c.width = 16
      c.height = 32
      const g = c.getContext('2d')!
      const legacy = img.height === 32
      const aw = slim ? 3 : 4
      const blit = (sx: number, sy: number, w: number, h: number, dx: number, dy: number) => g.drawImage(img, sx, sy, w, h, dx, dy, w, h)
      const part = (base: [number, number], over: [number, number] | null, w: number, h: number, dx: number, dy: number) => {
        blit(base[0], base[1], w, h, dx, dy)
        if (over && !legacy) blit(over[0], over[1], w, h, dx, dy)
      }
      part([8, 8], [40, 8], 8, 8, 4, 0)
      part([20, 20], [20, 36], 8, 12, 4, 8)
      part([44, 20], [44, 36], aw, 12, 4 - aw, 8)
      part(legacy ? [44, 20] : [36, 52], [52, 52], aw, 12, 12, 8)
      part([4, 20], [4, 36], 4, 12, 4, 20)
      part(legacy ? [4, 20] : [20, 52], [4, 52], 4, 12, 8, 20)
      resolve(c.toDataURL())
    }
    img.onerror = () => resolve('')
    img.src = src
  })
}

/** Slim (Alex) skins leave the last column of the right arm transparent. */
export function detectSlim(img: HTMLImageElement): boolean {
  if (img.height < 64) return false
  const c = document.createElement('canvas')
  c.width = img.width
  c.height = img.height
  const g = c.getContext('2d')!
  g.drawImage(img, 0, 0)
  return g.getImageData(55, 20, 1, 12).data.every((v, i) => i % 4 !== 3 || v === 0)
}
