<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { api, type JavaInfo, type Settings, type UpdateInfo } from '../api'
import ProgressLine from '../components/ProgressLine.vue'
import { background, guard, readDataUrl, state, toast } from '../store'

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
    toast('success', '설정을 저장했습니다')
  }
}
async function install(major: number) {
  toast('info', `Java ${major} 다운로드 중...`)
  const p = await guard(() => api.javaInstall(major))
  if (p) { toast('success', `Java ${major} 설치 완료`); s.value.java_path = p; scan() }
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
  if (upd.value && !upd.value.available) toast('success', `최신 버전입니다 (v${upd.value.current})`)
}
async function installUpdate() {
  toast('info', '업데이트를 다운로드합니다. 끝나면 런처가 다시 시작됩니다.')
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
          <h1>Setting</h1>
          <span class="spacer"></span>
          <span v-if="dirty" class="small" style="color: var(--warn)">저장하지 않은 변경 사항</span>
          <button class="btn primary" :disabled="!dirty" @click="save">저장</button>
        </div>

        <div class="set-grid">
          <section class="sect">
            <h3>계정</h3>
            <dl class="kv" style="margin: 0 0 12px">
              <dt>이름</dt><dd>{{ account?.name }}</dd>
              <dt>종류</dt><dd>{{ account?.kind === 'microsoft' ? 'Microsoft (정품)' : '오프라인' }}</dd>
            </dl>
            <div class="field">
              <label>Microsoft 앱 (client) ID</label>
              <input v-model="s.ms_client_id" placeholder="비워두면 내장된 LAVA LNCH ID 사용" />
              <div class="hint">Azure 앱 "LAVA LNCH"의 애플리케이션 ID. 리디렉션 URI는 http://localhost:9876 (퍼블릭 클라이언트/네이티브)이어야 합니다. 바꾼 뒤 저장하고 다시 로그인하세요.</div>
            </div>
            <button class="btn sm danger" @click="logout">로그아웃</button>
          </section>

          <section class="sect">
            <h3>배경화면</h3>
            <div class="bg-thumb" :style="{ backgroundImage: `url(${background()})` }"></div>
            <div class="row">
              <button class="btn sm" @click="picker?.click()">이미지 변경</button>
              <button class="btn sm" :disabled="!state.customBg" @click="resetBg">기본으로</button>
            </div>
            <input ref="picker" type="file" accept="image/png,image/jpeg,image/webp,image/gif" hidden @change="pickBg(($event.target as HTMLInputElement).files?.[0])" />
            <div class="hint small muted" style="margin-top: 8px">Single 화면과, 배경을 지정하지 않은 서버에 쓰입니다. 바로 적용됩니다.</div>
          </section>

          <section class="sect">
            <h3>Java</h3>
            <div class="field">
              <label>Java 실행 파일</label>
              <input v-model="s.java_path" placeholder="자동 (버전에 맞는 Java를 찾고 없으면 Temurin 설치)" />
            </div>
            <div class="field">
              <label>감지된 Java</label>
              <select @change="s.java_path = ($event.target as HTMLSelectElement).value">
                <option value="">{{ scanning ? '찾는 중...' : `${javas.length}개 발견 - 선택하세요` }}</option>
                <option v-for="j in javas" :key="j.path" :value="j.path">Java {{ j.major }} ({{ j.version }}) - {{ j.path }}</option>
              </select>
            </div>
            <div class="row">
              <button class="btn sm" @click="install(21)">Java 21 설치</button>
              <button class="btn sm" @click="install(17)">Java 17 설치</button>
              <button class="btn sm" @click="scan">다시 찾기</button>
            </div>
            <div class="hint small muted" style="margin-top: 8px">1.20.5 이상은 Java 21, 1.19 ~ 1.20.4는 Java 17이 필요합니다.</div>
          </section>

          <section class="sect">
            <h3>메모리</h3>
            <div class="field">
              <label class="row-between"><span>최소 (-Xms)</span><span class="range-val">{{ gb(s.ram_min) }}</span></label>
              <input v-model.number="s.ram_min" type="range" min="512" max="16384" step="256" />
            </div>
            <div class="field">
              <label class="row-between"><span>최대 (-Xmx)</span><span class="range-val">{{ gb(s.ram_max) }}</span></label>
              <input v-model.number="s.ram_max" type="range" min="1024" max="32768" step="256" />
            </div>
            <div class="field">
              <label>GC 프리셋</label>
              <select v-model="s.jvm_preset">
                <option value="none">없음 (JVM 기본값)</option>
                <option value="aikar">Aikar's flags (G1GC, 추천)</option>
                <option value="zgc">ZGC</option>
                <option value="shenandoah">Shenandoah</option>
              </select>
            </div>
            <div class="field">
              <label>추가 JVM 인자</label>
              <input v-model="s.extra_jvm" placeholder="-Dfoo=bar" />
            </div>
          </section>

          <section class="sect">
            <h3>업데이트</h3>
            <dl class="kv" style="margin: 0 0 12px">
              <dt>현재 버전</dt><dd>v{{ upd?.current ?? '?' }}</dd>
              <template v-if="upd"><dt>최신 릴리즈</dt><dd>v{{ upd.latest || '-' }} <span class="small muted">({{ upd.repo }})</span></dd></template>
            </dl>
            <div class="field">
              <label>릴리즈 저장소</label>
              <input v-model="s.update_repo" :placeholder="defaultRepo ? `기본값: ${defaultRepo}` : 'owner/repo'" />
              <div class="hint">GitHub "owner/repo" 또는 저장소 주소. 비워두면 기본값을 씁니다. 직접 만든 런처라면 여기를 바꾸세요.</div>
            </div>
            <label class="check" style="margin-bottom: 12px"><input v-model="s.auto_update" type="checkbox" /> 시작할 때 업데이트 확인</label>
            <div class="row">
              <button class="btn sm" :disabled="checking" @click="checkUpdate">{{ checking ? '확인 중...' : '지금 확인' }}</button>
              <button v-if="upd?.available && upd.can_install" class="btn sm primary" :disabled="state.progress.active" @click="installUpdate">v{{ upd.latest }} 설치</button>
            </div>
            <div v-if="upd?.available && !upd.can_install" class="hint small muted" style="margin-top: 8px">개발 빌드에서는 자동 설치를 하지 않습니다.</div>
            <div style="margin-top: 8px"><ProgressLine /></div>
          </section>

          <section class="sect">
            <h3>런처</h3>
            <div class="field">
              <label>게임을 시작하면</label>
              <select v-model="s.launcher_behavior">
                <option value="hide">런처 숨기기 (게임 종료 시 다시 표시)</option>
                <option value="keep">런처 유지 (게임 로그 확인 가능)</option>
                <option value="close">런처 종료</option>
              </select>
            </div>
            <div class="row">
              <button class="btn sm" @click="api.openFolder()">데이터 폴더 열기</button>
              <button class="btn sm" :disabled="!state.log.length" @click="state.showLog = true">마지막 게임 로그</button>
            </div>
          </section>
        </div>
      </div>
    </div>
  </div>
</template>
