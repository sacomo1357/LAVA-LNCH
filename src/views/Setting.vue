<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { api, type JavaInfo, type Settings, type UpdateInfo } from '../api'
import ProgressLine from '../components/ProgressLine.vue'
import { background, guard, readDataUrl, state, toast } from '../store'
import { LANGS, lang, t } from '../i18n'

const s = ref<Settings>({ ...state.settings! })
const javas = ref<JavaInfo[]>([])
const scanning = ref(false)
const picker = ref<HTMLInputElement>()
const account = computed(() => state.settings?.account)
const dirty = computed(() => JSON.stringify({ ...s.value, account: null }) !== JSON.stringify({ ...state.settings, account: null }))

const gb = (mb: number) => (mb / 1024).toFixed(1) + ' GB'

async function scan() {
  scanning.value = true
  javas.value = (await guard(api.javaDetect)) ?? []
  scanning.value = false
}
onMounted(scan)

async function save() {
  if (s.value.ram_max < s.value.ram_min) s.value.ram_max = s.value.ram_min
  const r = await guard(() => api.saveSettings(s.value))
  if (r) {
    state.settings = r
    s.value = { ...r }
    toast('success', t('setting.saved'))
  }
}
async function install(major: number) {
  toast('info', t('setting.javaDownloading', { major }))
  const p = await guard(() => api.javaInstall(major))
  if (p) { toast('success', t('setting.javaInstalled', { major })); s.value.java_path = p; scan() }
}
async function pickBg(file?: File) {
  if (!file) return
  const url = await guard(() => readDataUrl(file))
  if (!url) return
  if ((await guard(() => api.bgSet(url))) !== undefined) state.customBg = url
}
async function resetBg() {
  if ((await guard(() => api.bgSet(null))) !== undefined) state.customBg = null
}
const defaultRepo = ref('')
const upd = ref<UpdateInfo | null>(null)
const checking = ref(false)
onMounted(async () => {
  defaultRepo.value = await api.updateDefaultRepo().catch(() => '')
  upd.value = await api.updateCheck().catch(() => null)
})
async function checkUpdate() {
  if (dirty.value) await save() // the check reads the saved repo
  checking.value = true
  upd.value = (await guard(api.updateCheck)) ?? null
  checking.value = false
  if (upd.value && !upd.value.available) toast('success', t('setting.upToDate', { version: upd.value.current }))
}
async function installUpdate() {
  toast('info', t('setting.updateDownloading'))
  await guard(api.updateInstall)
}

async function logout() {
  const r = await guard(api.authLogout)
  if (r) state.settings = r
}
</script>

<template>
  <div class="scene" :style="{ backgroundImage: `url(${background()})` }">
    <div class="framed">
      <div class="page">
        <div class="page-head">
          <h1>{{ t('tab.setting') }}</h1>
          <span class="spacer"></span>
          <span v-if="dirty" class="small" style="color: var(--warn)">{{ t('setting.unsaved') }}</span>
          <button class="btn primary" :disabled="!dirty" @click="save">{{ t('common.save') }}</button>
        </div>

        <div class="set-grid">
          <section class="sect">
            <h3>{{ t('setting.account') }}</h3>
            <dl class="kv" style="margin: 0 0 12px">
              <dt>{{ t('common.name') }}</dt><dd>{{ account?.name }}</dd>
              <dt>{{ t('setting.kind') }}</dt><dd>{{ account?.kind === 'microsoft' ? t('setting.microsoftOwned') : t('common.offline') }}</dd>
            </dl>
            <div class="field">
              <label>{{ t('setting.msClientId') }}</label>
              <input v-model="s.ms_client_id" :placeholder="t('setting.msClientIdPlaceholder')" />
              <div class="hint">{{ t('setting.msClientIdHint') }}</div>
            </div>
            <button class="btn sm danger" @click="logout">{{ t('setting.logout') }}</button>
          </section>

          <section class="sect">
            <h3>{{ t('setting.background') }}</h3>
            <div class="bg-thumb" :style="{ backgroundImage: `url(${background()})` }"></div>
            <div class="row">
              <button class="btn sm" @click="picker?.click()">{{ t('setting.changeImage') }}</button>
              <button class="btn sm" :disabled="!state.customBg" @click="resetBg">{{ t('setting.resetImage') }}</button>
            </div>
            <input ref="picker" type="file" accept="image/png,image/jpeg,image/webp,image/gif" hidden @change="pickBg(($event.target as HTMLInputElement).files?.[0])" />
            <div class="hint small muted" style="margin-top: 8px">{{ t('setting.bgHint') }}</div>
          </section>

          <section class="sect">
            <h3>Java</h3>
            <div class="field">
              <label>{{ t('setting.javaPath') }}</label>
              <input v-model="s.java_path" :placeholder="t('setting.javaPathPlaceholder')" />
            </div>
            <div class="field">
              <label>{{ t('setting.javaDetected') }}</label>
              <select @change="s.java_path = ($event.target as HTMLSelectElement).value">
                <option value="">{{ scanning ? t('setting.javaScanning') : t('setting.javaFound', { n: javas.length }) }}</option>
                <option v-for="j in javas" :key="j.path" :value="j.path">Java {{ j.major }} ({{ j.version }}) - {{ j.path }}</option>
              </select>
            </div>
            <div class="row">
              <button class="btn sm" @click="install(21)">{{ t('setting.javaInstall', { major: 21 }) }}</button>
              <button class="btn sm" @click="install(17)">{{ t('setting.javaInstall', { major: 17 }) }}</button>
              <button class="btn sm" @click="scan">{{ t('setting.rescan') }}</button>
            </div>
            <div class="hint small muted" style="margin-top: 8px">{{ t('setting.javaHint') }}</div>
          </section>

          <section class="sect">
            <h3>{{ t('setting.memory') }}</h3>
            <div class="field">
              <label class="row-between"><span>{{ t('setting.ramMin') }}</span><span class="range-val">{{ gb(s.ram_min) }}</span></label>
              <input v-model.number="s.ram_min" type="range" min="512" max="16384" step="256" />
            </div>
            <div class="field">
              <label class="row-between"><span>{{ t('setting.ramMax') }}</span><span class="range-val">{{ gb(s.ram_max) }}</span></label>
              <input v-model.number="s.ram_max" type="range" min="1024" max="32768" step="256" />
            </div>
            <div class="field">
              <label>{{ t('setting.gcPreset') }}</label>
              <select v-model="s.jvm_preset">
                <option value="none">{{ t('setting.gcNone') }}</option>
                <option value="aikar">{{ t('setting.gcAikar') }}</option>
                <option value="zgc">ZGC</option>
                <option value="shenandoah">Shenandoah</option>
              </select>
            </div>
            <div class="field">
              <label>{{ t('setting.extraJvm') }}</label>
              <input v-model="s.extra_jvm" placeholder="-Dfoo=bar" />
            </div>
          </section>

          <section class="sect">
            <h3>{{ t('setting.update') }}</h3>
            <dl class="kv" style="margin: 0 0 12px">
              <dt>{{ t('setting.currentVersion') }}</dt><dd>v{{ upd?.current ?? '?' }}</dd>
              <template v-if="upd"><dt>{{ t('setting.latestRelease') }}</dt><dd>v{{ upd.latest || '-' }} <span class="small muted">({{ upd.repo }})</span></dd></template>
            </dl>
            <div class="field">
              <label>{{ t('setting.releaseRepo') }}</label>
              <input v-model="s.update_repo" :placeholder="defaultRepo ? t('setting.repoDefault', { repo: defaultRepo }) : 'owner/repo'" />
              <div class="hint">{{ t('setting.repoHint') }}</div>
            </div>
            <label class="check" style="margin-bottom: 12px"><input v-model="s.auto_update" type="checkbox" /> {{ t('setting.autoUpdate') }}</label>
            <div class="row">
              <button class="btn sm" :disabled="checking" @click="checkUpdate">{{ checking ? t('common.checking') : t('setting.checkNow') }}</button>
              <button v-if="upd?.available && upd.can_install" class="btn sm primary" :disabled="state.progress.active" @click="installUpdate">{{ t('setting.installVersion', { version: upd.latest }) }}</button>
            </div>
            <div v-if="upd?.available && !upd.can_install" class="hint small muted" style="margin-top: 8px">{{ t('setting.devBuild') }}</div>
            <div style="margin-top: 8px"><ProgressLine /></div>
          </section>

          <section class="sect">
            <h3>{{ t('setting.launcher') }}</h3>
            <div class="field">
              <label>{{ t('setting.language') }}</label>
              <select v-model="lang">
                <option v-for="l in LANGS" :key="l.id" :value="l.id">{{ l.label }}</option>
              </select>
            </div>
            <div class="field">
              <label>{{ t('setting.onLaunch') }}</label>
              <select v-model="s.launcher_behavior">
                <option value="hide">{{ t('setting.behaviorHide') }}</option>
                <option value="keep">{{ t('setting.behaviorKeep') }}</option>
                <option value="close">{{ t('setting.behaviorClose') }}</option>
              </select>
            </div>
            <div class="row">
              <button class="btn sm" @click="api.openFolder()">{{ t('setting.openData') }}</button>
              <button class="btn sm" :disabled="!state.log.length" @click="state.showLog = true">{{ t('setting.lastLog') }}</button>
            </div>
          </section>
        </div>
      </div>
    </div>
  </div>
</template>
