<script setup lang="ts">
import {
  Archive,
  ArrowRight,
  CheckCircle2,
  CircleGauge,
  Database,
  File,
  Folder,
  Hash,
  Info,
  List,
  ListChecks,
  Play,
  Search,
  Settings,
  Tag,
  Trash2,
  Users,
  Zap
} from 'lucide-vue-next'
import { computed, nextTick, onMounted, onUnmounted, reactive, ref, watch } from 'vue'
import {
  cancelScan,
  clearCache,
  loadSettings,
  onScanActivity,
  onScanCancelled,
  onScanCompleted,
  onScanFailed,
  pickFolder,
  pickFolders,
  revealFile,
  saveCacheLimit,
  saveHashAlgorithm,
  saveQuarantineDestination,
  saveScanMode,
  saveTextFile,
  startScan,
  volumeDestinations,
  type ActivityEvent,
  type CleanReport,
  type HashAlgorithmId,
  type ScanModeId,
  type VolumeDestination
} from './lib/native'
import {
  compactHash,
  compactPath,
  formatActivityLog,
  formatBytes,
  formatDuplicateRelationsLog,
  progressFromReport,
  relationKindLabel
} from './lib/format'

type DialogName =
  | 'none'
  | 'cacheConfirm'
  | 'scanMode'
  | 'algorithm'
  | 'paths'
  | 'cacheLimit'
  | 'quarantine'
  | 'scanConfirm'
  | 'relations'

type RelationFilter = 'all' | 'sameNameAndSize' | 'sameSizeAndHash'

const algorithms: Array<{ id: HashAlgorithmId; label: string; description: string }> = [
  { id: 'BLAKE3', label: 'BLAKE3', description: '기본값, 빠른 일반 검사에 적합' },
  { id: 'SHA256', label: 'SHA-256', description: '범용 호환성이 높은 256비트 해시' },
  { id: 'SHA512', label: 'SHA-512', description: '긴 다이제스트가 필요한 검사' },
  { id: 'MD5', label: 'MD5', description: '레거시 비교용' }
]

const scanModes: Array<{ id: ScanModeId; label: string; description: string }> = [
  { id: 'FAST', label: '빠른 일반 모드', description: '같은 용량 파일만 해시하고 캐시를 재사용' },
  { id: 'FULL_HASH', label: '전체 해시 모드', description: '모든 파일을 해시 대상으로 포함' },
  { id: 'REHASH', label: '재계산 모드', description: '캐시 없이 후보 해시를 다시 계산' }
]

const relationFilters: Array<{ id: RelationFilter; label: string }> = [
  { id: 'all', label: '전체' },
  { id: 'sameNameAndSize', label: '같은 이름+용량' },
  { id: 'sameSizeAndHash', label: '다른 이름+용량+해시' }
]

const state = reactive({
  paths: [] as string[],
  cacheLimitMb: 256,
  cacheLimitConfigured: false,
  cacheLimitInput: '256',
  algorithm: 'BLAKE3' as HashAlgorithmId,
  algorithmConfigured: false,
  scanMode: 'FAST' as ScanModeId,
  scanModeConfigured: false,
  destinations: [] as VolumeDestination[],
  report: null as CleanReport | null,
  activityEvents: [] as ActivityEvent[],
  activityLogEvents: [] as ActivityEvent[],
  scanProgress: null as { progress: number; completed: number; total: number } | null,
  statusMessage: '중복 파일을 검사할 디렉터리를 선택하십시오.',
  errorMessage: '',
  running: false,
  scanId: 0,
  dialog: 'none' as DialogName,
  selectedPaths: [] as string[],
  relationFilter: 'all' as RelationFilter
})

const pending = ref(false)
const activityBox = ref<HTMLElement | null>(null)
const lastActivityWheelAt = ref(0)
const unlisteners: Array<() => void> = []

const selectedScanMode = computed(() => scanModes.find((mode) => mode.id === state.scanMode) ?? scanModes[0])
const selectedAlgorithm = computed(() => algorithms.find((algorithm) => algorithm.id === state.algorithm) ?? algorithms[0])
const hasPaths = computed(() => state.paths.length > 0)
const configuredDestinationCount = computed(() => state.destinations.filter((destination) => destination.configured).length)
const quarantineRequired = computed(() => hasPaths.value && state.destinations.some((destination) => !destination.configured))
const canStartScan = computed(() => hasPaths.value && !quarantineRequired.value)
const quarantineConfigured = computed(() => state.destinations.length > 0 && configuredDestinationCount.value === state.destinations.length)
const pathTitle = computed(() => {
  if (state.paths.length === 0) {
    return '선택된 경로 없음'
  }

  return state.paths.length === 1 ? state.paths[0] : `${state.paths.length}개 경로 선택`
})
const pathBadge = computed(() => {
  if (state.paths.length === 0) {
    return '선택된 경로 없음'
  }

  return state.paths.length === 1 ? compactPath(state.paths[0], 24) : `${state.paths.length}개 경로 선택`
})
const quarantineTitle = computed(() => {
  if (state.destinations.length === 0) {
    return '미지정'
  }

  return configuredDestinationCount.value === state.destinations.length
    ? `${configuredDestinationCount.value}개 폴더 지정`
    : `${configuredDestinationCount.value}/${state.destinations.length} 지정`
})
const progress = computed(() => {
  if (state.report) {
    return state.running ? progressFromReport(state.report) : 1
  }

  return state.scanProgress?.progress ?? 0
})
const progressValue = computed(() => Math.min(1, Math.max(0, Number.isFinite(progress.value) ? progress.value : 0)))
const progressText = computed(() => `${(progressValue.value * 100).toFixed(1)}%`)
const processed = computed(() => state.report?.scanned_files ?? state.scanProgress?.completed ?? 0)
const total = computed(() => state.report?.scanned_files ?? state.scanProgress?.total ?? 0)
const duplicateRelations = computed(() => state.report?.duplicate_relations ?? [])
const filteredRelations = computed(() =>
  duplicateRelations.value.filter((relation) => {
    if (state.relationFilter === 'all') {
      return true
    }

    const sameName = relation.kind === 'SameNameAndSize' || relation.kind === 'SAME_NAME_AND_SIZE'
    return state.relationFilter === 'sameNameAndSize' ? sameName : !sameName
  })
)
const statusLabel = computed(() => {
  if (state.running) {
    return '진행 중'
  }

  if (state.report) {
    return '완료'
  }

  return state.errorMessage ? '오류' : '대기 중'
})
const actionHint = computed(() => {
  if (!hasPaths.value) {
    return '중복 파일을 검사할 디렉터리를 선택하십시오.'
  }

  if (state.running) {
    return '검사 및 중복 제거를 실행 중입니다.'
  }

  if (quarantineRequired.value) {
    return '검사 전 모든 검사 폴더의 보관 폴더를 지정하십시오.'
  }

  return '선택한 디렉터리에서 중복 제거를 시작할 수 있습니다.'
})

onMounted(async () => {
  window.addEventListener('keydown', handleDialogKeydown)
  await initializeSettings()
  unlisteners.push(await onScanActivity(handleActivity))
  unlisteners.push(await onScanCompleted(handleCompleted))
  unlisteners.push(await onScanFailed(handleFailed))
  unlisteners.push(await onScanCancelled(handleCancelled))
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleDialogKeydown)

  for (const unlisten of unlisteners) {
    unlisten()
  }
})

watch(() => state.activityEvents.length, scrollActivityToBottom)

async function initializeSettings() {
  try {
    const settings = await loadSettings()
    state.cacheLimitMb = settings.cache_limit_mb
    state.cacheLimitInput = String(settings.cache_limit_mb)
    state.cacheLimitConfigured = settings.cache_limit_configured
    state.algorithm = normalizeAlgorithm(settings.algorithm)
    state.algorithmConfigured = settings.algorithm_configured
    state.scanMode = normalizeScanMode(settings.scan_mode)
    state.scanModeConfigured = settings.scan_mode_configured
  } catch (error) {
    setFailure(error)
  }
}

async function chooseFolders() {
  if (state.running) {
    return
  }

  try {
    const folders = await pickFolders()

    if (folders.length === 0) {
      return
    }

    const completed = state.report != null
    const previousCount = completed ? 0 : state.paths.length
    const unique = Array.from(new Set(completed ? folders : [...state.paths, ...folders]))
    const addedCount = unique.length - previousCount

    if (addedCount === 0) {
      state.statusMessage = '선택한 디렉터리는 이미 검사 목록에 포함되어 있습니다.'
      await refreshDestinations()
      return
    }

    state.paths = unique
    state.report = null
    state.statusMessage = completed
      ? `${unique.length}개 디렉터리를 새 검사 목록으로 선택했습니다.`
      : `${addedCount}개 디렉터리를 추가했습니다. 현재 ${unique.length}개 디렉터리가 검사 목록에 있습니다.`
    await refreshDestinations()
  } catch (error) {
    setFailure(error)
  }
}

async function resetDestinations() {
  if (state.paths.length === 0) {
    state.destinations = []
    return
  }

  try {
    await refreshDestinations()
  } catch (error) {
    setFailure(error)
  }
}

async function refreshDestinations() {
  state.destinations = state.paths.length === 0 ? [] : await volumeDestinations(state.paths)
}

async function removeSelectedPaths() {
  if (state.selectedPaths.length === 0 || state.running) {
    return
  }

  const selected = new Set(state.selectedPaths)
  state.paths = state.paths.filter((path) => !selected.has(path))
  state.selectedPaths = []
  state.report = null
  state.statusMessage = state.paths.length === 0
    ? '중복 파일을 검사할 디렉터리를 선택하십시오.'
    : `${state.paths.length}개 디렉터리가 검사 목록에 남아 있습니다.`
  await resetDestinations()

  if (state.paths.length === 0) {
    closeDialog()
  }
}

async function updateScanMode(value: ScanModeId) {
  try {
    await saveScanMode(value)
    state.scanMode = value
    state.scanModeConfigured = true
    state.statusMessage = `검사 모드를 ${scanModes.find((mode) => mode.id === value)?.label ?? value}로 저장했습니다.`
  } catch (error) {
    setFailure(error)
  }
}

async function updateAlgorithm(value: HashAlgorithmId) {
  try {
    await saveHashAlgorithm(value)
    state.algorithm = value
    state.algorithmConfigured = true
    state.statusMessage = `비교 기준을 ${algorithms.find((algorithm) => algorithm.id === value)?.label ?? value}로 저장했습니다.`
  } catch (error) {
    setFailure(error)
  }
}

async function updateCacheLimit() {
  const value = Number.parseInt(state.cacheLimitInput, 10)

  if (!Number.isFinite(value)) {
    state.statusMessage = '캐시 제한은 숫자로 입력하십시오.'
    return
  }

  if (value < 16) {
    state.statusMessage = '캐시 제한은 16 MB 이상으로 입력하십시오.'
    return
  }

  try {
    const result = await saveCacheLimit(value)
    state.cacheLimitMb = value
    state.cacheLimitConfigured = true
    state.statusMessage =
      result.pruned > 0
        ? `SQLite 캐시 제한을 ${value} MB로 저장하고 오래된 해시 ${result.pruned}개를 정리했습니다.`
        : `SQLite 캐시 제한을 ${value} MB로 저장했습니다.`
    closeDialog()
  } catch (error) {
    setFailure(error)
  }
}

async function chooseQuarantine(destination: VolumeDestination) {
  if (state.running) {
    return
  }

  try {
    const folder = await pickFolder(destination.configured ? destination.target_path : destination.root_path)

    if (!folder) {
      return
    }

    await saveQuarantineDestination(destination.volume_key, folder)
    await refreshDestinations()
    state.statusMessage = '보관 폴더를 저장했습니다.'
  } catch (error) {
    setFailure(error)
  }
}

async function confirmClearCache() {
  try {
    const result = await clearCache()
    state.report = null
    state.statusMessage =
      result.removed > 0 ? `SQLite 캐시 파일 ${result.removed}개를 삭제했습니다.` : '삭제할 SQLite 캐시 파일이 없습니다.'
    closeDialog()
  } catch (error) {
    setFailure(error)
  }
}

async function runOrStop() {
  if (state.running) {
    await cancelCurrentScan()
    return
  }

  openScanConfirm()
}

function openScanConfirm() {
  if (!hasPaths.value) {
    state.statusMessage = '중복 파일을 검사할 디렉터리를 선택하십시오.'
    return
  }

  if (quarantineRequired.value) {
    state.statusMessage = '모든 검사 폴더의 보관 폴더를 먼저 지정하십시오.'
    return
  }

  openDialog('scanConfirm')
}

async function confirmStartScan() {
  closeDialog()
  pending.value = true
  state.running = true
  state.report = null
  state.errorMessage = ''
  state.activityEvents = []
  state.activityLogEvents = []
  state.scanProgress = null
  state.statusMessage = '검사 및 중복 제거를 실행 중입니다.'

  try {
    const run = await startScan(state.paths, state.algorithm, state.scanMode)

    if (state.running && state.scanId === 0) {
      state.scanId = run.scan_id
    }
  } catch (error) {
    state.running = false
    setFailure(error)
  } finally {
    pending.value = false
  }
}

async function cancelCurrentScan() {
  if (state.scanId === 0) {
    state.running = false
    state.statusMessage = '사용자가 검사를 중지했습니다.'
    return
  }

  try {
    await cancelScan(state.scanId)
    state.statusMessage = '검사 중지를 요청했습니다.'
  } catch (error) {
    setFailure(error)
  }
}

async function saveActivityLog() {
  if (state.activityLogEvents.length === 0) {
    state.statusMessage = '저장할 실시간 작업 로그가 없습니다.'
    return
  }

  await saveText('hash-killer-activity.log', formatActivityLog(state.activityLogEvents), '실시간 작업 로그를 저장했습니다.')
}

async function saveRelationsLog() {
  if (duplicateRelations.value.length === 0) {
    state.statusMessage = '저장할 중복 관계 로그가 없습니다.'
    return
  }

  await saveText(
    'hash-killer-duplicates.log',
    formatDuplicateRelationsLog(duplicateRelations.value),
    '중복 관계 로그를 저장했습니다.'
  )
}

async function saveText(suggestedName: string, contents: string, successMessage: string) {
  try {
    const result = await saveTextFile(suggestedName, contents)

    if (result.ok) {
      state.statusMessage = successMessage
    }
  } catch (error) {
    setFailure(error)
  }
}

async function openFile(path: string) {
  try {
    await revealFile(path)
    state.statusMessage = '파일 위치를 열었습니다.'
  } catch (error) {
    setFailure(error)
  }
}

function handleActivity(payload: { scan_id: number; event: ActivityEvent }) {
  if (!acceptsScanPayload(payload.scan_id)) {
    return
  }

  pushActivity(payload.event)
}

function handleCompleted(payload: { scan_id: number; report: CleanReport }) {
  if (!acceptsScanPayload(payload.scan_id)) {
    return
  }

  pushActivity({ stage: '완료', detail: '검사가 완료되었습니다.', path: null, progress: 1, completed: payload.report.scanned_files, total: payload.report.scanned_files })
  state.report = payload.report
  state.running = false
  state.scanId = 0
  state.statusMessage = '검사가 완료되었습니다.'
}

function handleFailed(payload: { scan_id: number; message: string }) {
  if (!acceptsScanPayload(payload.scan_id)) {
    return
  }

  pushActivity({ stage: '오류', detail: payload.message, path: null, progress: null, completed: null, total: null })
  state.running = false
  state.scanId = 0
  state.errorMessage = payload.message
  state.statusMessage = payload.message
}

function handleCancelled(payload: { scan_id: number }) {
  if (!acceptsScanPayload(payload.scan_id)) {
    return
  }

  pushActivity({ stage: '중지', detail: '사용자가 검사를 중지했습니다.', path: null, progress: null, completed: null, total: null })
  state.running = false
  state.scanId = 0
  state.statusMessage = '사용자가 검사를 중지했습니다.'
}

function acceptsScanPayload(scanId: number) {
  if (scanId === state.scanId) {
    return true
  }

  if (state.running && state.scanId === 0) {
    state.scanId = scanId
    return true
  }

  return false
}

function pushActivity(event: ActivityEvent) {
  state.activityLogEvents = [...state.activityLogEvents, event]
  state.activityEvents = state.activityLogEvents.slice(-200)

  if (event.progress != null && event.completed != null && event.total != null) {
    state.scanProgress = {
      progress: event.progress,
      completed: event.completed,
      total: event.total
    }
  }
}

async function scrollActivityToBottom() {
  await nextTick()
  const box = activityBox.value

  if (box) {
    box.scrollTop = box.scrollHeight
  }
}

function handleActivityWheel(event: WheelEvent) {
  const box = activityBox.value

  if (!box || window.innerHeight > 700 || state.activityEvents.length === 0) {
    return
  }

  event.preventDefault()

  const now = Date.now()

  if (now - lastActivityWheelAt.value < 140) {
    return
  }

  lastActivityWheelAt.value = now

  const step = box.clientHeight
  const maxIndex = Math.max(0, Math.ceil((box.scrollHeight - box.clientHeight) / step))
  const currentIndex = Math.round(box.scrollTop / step)
  const direction = event.deltaY > 0 ? 1 : -1
  const nextIndex = Math.min(maxIndex, Math.max(0, currentIndex + direction))
  box.scrollTop = nextIndex * step
}

function handleDialogKeydown(event: KeyboardEvent) {
  if (event.key !== 'Escape' || state.dialog === 'none') {
    return
  }

  event.preventDefault()
  closeDialog()
}

function openDialog(dialog: DialogName) {
  if (state.running && dialog === 'cacheConfirm') {
    return
  }

  if (dialog === 'cacheLimit') {
    state.cacheLimitInput = String(state.cacheLimitMb)
  }

  if (dialog === 'paths') {
    state.selectedPaths = []
  }

  if (dialog === 'relations') {
    state.relationFilter = 'all'
  }

  state.dialog = dialog
}

function closeDialog() {
  state.dialog = 'none'
}

function togglePathSelection(path: string, checked: boolean) {
  if (checked) {
    state.selectedPaths = Array.from(new Set([...state.selectedPaths, path]))
  } else {
    state.selectedPaths = state.selectedPaths.filter((selected) => selected !== path)
  }
}

function handlePathSelection(path: string, event: Event) {
  togglePathSelection(path, (event.target as HTMLInputElement).checked)
}

function relationFilterCount(filter: RelationFilter) {
  return duplicateRelations.value.filter((relation) => {
    const sameName = relation.kind === 'SameNameAndSize' || relation.kind === 'SAME_NAME_AND_SIZE'

    if (filter === 'all') {
      return true
    }

    return filter === 'sameNameAndSize' ? sameName : !sameName
  }).length
}

function setFailure(error: unknown) {
  const message = error instanceof Error ? error.message : String(error)
  state.errorMessage = message
  state.statusMessage = message
}

function normalizeAlgorithm(value: string): HashAlgorithmId {
  const normalized = value.toUpperCase()

  if (normalized === 'SHA256' || normalized === 'SHA-256') {
    return 'SHA256'
  }

  if (normalized === 'SHA512' || normalized === 'SHA-512') {
    return 'SHA512'
  }

  if (normalized === 'MD5') {
    return 'MD5'
  }

  return 'BLAKE3'
}

function normalizeScanMode(value: string): ScanModeId {
  const normalized = value.toUpperCase()

  if (normalized === 'FULL_HASH' || normalized === 'FULL') {
    return 'FULL_HASH'
  }

  if (normalized === 'REHASH' || normalized === 'RECALCULATE') {
    return 'REHASH'
  }

  return 'FAST'
}
</script>

<template>
  <main class="app-shell">
    <section class="top-panel">
      <div class="top-title">
        <span class="icon-box top-icon"><Folder :size="18"  /></span>
        <h1 :title="pathTitle">{{ pathTitle }}</h1>
      </div>
      <button class="primary-outline-button folder-button" :disabled="state.running" @click="chooseFolders">
        <Folder :size="16"  />
        <span>폴더 선택</span>
      </button>
    </section>

    <div class="dashboard-grid">
      <section class="section-panel settings-panel">
          <header class="section-header">
            <span class="icon-box"><Settings :size="16"  /></span>
            <h2>검사 설정</h2>
          </header>
          <div class="setting-list">
            <div class="setting-row">
              <span class="row-icon"><CircleGauge :size="16"  /></span>
              <span class="row-label">검사 모드</span>
              <span class="row-value">{{ selectedScanMode.label }}</span>
              <button class="small-button" :class="{ unconfigured: !state.scanModeConfigured }" :disabled="state.running" @click="openDialog('scanMode')">설정</button>
            </div>
            <div class="setting-row">
              <span class="row-icon"><Tag :size="16"  /></span>
              <span class="row-label">비교 기준</span>
              <span class="row-value">{{ selectedAlgorithm.label }}</span>
              <button class="small-button" :class="{ unconfigured: !state.algorithmConfigured }" :disabled="state.running" @click="openDialog('algorithm')">설정</button>
            </div>
            <div class="setting-row">
              <span class="row-icon"><Folder :size="16"  /></span>
              <span class="row-label">검사 경로</span>
              <span class="row-value" :title="pathTitle">{{ pathBadge }}</span>
              <button class="small-button" :class="{ unconfigured: !hasPaths }" :disabled="state.running" @click="openDialog('paths')">설정</button>
            </div>
            <div class="setting-row">
              <span class="row-icon"><Database :size="16"  /></span>
              <span class="row-label">캐시 제한</span>
              <span class="row-value">{{ state.cacheLimitMb }} MB</span>
              <button class="small-button" :class="{ unconfigured: !state.cacheLimitConfigured }" :disabled="state.running" @click="openDialog('cacheLimit')">설정</button>
            </div>
            <div class="setting-row">
              <span class="row-icon"><Archive :size="16"  /></span>
              <span class="row-label">보관 폴더</span>
              <span class="row-value">{{ quarantineTitle }}</span>
              <button class="small-button" :class="{ unconfigured: !quarantineConfigured }" :disabled="state.running" @click="openDialog('quarantine')">설정</button>
            </div>
          </div>
        </section>

      <section class="section-panel progress-panel">
        <header class="section-header">
          <span class="icon-box"><Zap :size="16"  /></span>
          <h2>진행 상태</h2>
        </header>
        <div class="progress-card">
          <div class="progress-main">
            <div class="progress-head">
              <strong>{{ progressText }}</strong>
              <span>{{ processed }}/{{ total }} 파일 처리</span>
            </div>
            <div class="progress-track">
              <div class="progress-bar" :class="{ indeterminate: state.running && !state.scanProgress }" :style="{ width: progressText }"></div>
            </div>
          </div>
          <div class="progress-meta">
            <div class="progress-meta-item">
              <File :size="16"  />
              <div>
                <span>처리된 파일</span>
                <strong>{{ processed }} / {{ total }}</strong>
              </div>
            </div>
            <div class="progress-meta-item">
              <CircleGauge :size="16"  />
              <div>
                <span>총 검사 시간</span>
                <strong>{{ state.report ? '완료' : '대기' }}</strong>
              </div>
            </div>
            <div class="progress-meta-item">
              <CheckCircle2 :size="16"  />
              <div>
                <span>상태</span>
                <strong>{{ statusLabel }}</strong>
              </div>
            </div>
          </div>
        </div>
      </section>

      <section class="section-panel activity-panel">
      <header class="section-header activity-header">
        <div class="header-title">
          <span class="icon-box"><List :size="16"  /></span>
          <h2>실시간 작업</h2>
        </div>
        <button class="log-button" :disabled="state.activityLogEvents.length === 0" @click="saveActivityLog">로그 저장</button>
      </header>
      <div ref="activityBox" class="activity-box" @wheel="handleActivityWheel">
        <div v-if="state.activityEvents.length === 0" class="activity-empty">
          <span class="activity-dot"></span>
          <span><strong>대기 중</strong> 검사를 시작하면 현재 처리 중인 작업과 파일이 표시됩니다.</span>
        </div>
        <div v-else class="activity-list">
          <div v-for="(event, index) in state.activityEvents" :key="`${index}-${event.stage}-${event.detail}`" class="activity-row">
            <span class="activity-dot"></span>
            <div>
              <p><strong>{{ event.stage }}</strong> {{ event.detail }}</p>
              <small v-if="event.path" :title="event.path">{{ event.path }}</small>
            </div>
          </div>
        </div>
      </div>
    </section>

      <section class="section-panel summary-panel">
        <header class="section-header">
          <span class="icon-box"><ListChecks :size="16"  /></span>
          <h2>결과 요약</h2>
        </header>
        <div class="summary-body">
          <div class="stat-grid">
            <article class="stat-tile">
              <span class="stat-icon blue"><Search :size="20"  /></span>
              <span class="stat-label">스캔</span>
              <strong>{{ state.report?.scanned_files ?? 0 }}</strong>
            </article>
            <article class="stat-tile">
              <span class="stat-icon purple"><List :size="20"  /></span>
              <span class="stat-label">후보</span>
              <strong>{{ state.report?.candidate_files ?? 0 }}</strong>
            </article>
            <article class="stat-tile">
              <span class="stat-icon green"><Hash :size="20"  /></span>
              <span class="stat-label">해시</span>
              <strong>{{ state.report?.hashed_files ?? 0 }}</strong>
            </article>
            <article class="stat-tile">
              <span class="stat-icon orange"><Database :size="20"  /></span>
              <span class="stat-label">캐시</span>
              <strong>{{ state.report?.reused_hashes ?? 0 }}</strong>
            </article>
            <article class="stat-tile">
              <span class="stat-icon yellow"><Users :size="20"  /></span>
              <span class="stat-label">그룹</span>
              <strong>{{ state.report?.duplicate_groups ?? 0 }}</strong>
            </article>
            <article class="stat-tile delete-tile">
              <span class="stat-icon red"><Trash2 :size="20"  /></span>
              <span class="stat-label">분류</span>
              <strong>{{ state.report?.deleted_files ?? 0 }}개 / {{ formatBytes(state.report?.reclaimed_bytes ?? 0) }}</strong>
            </article>
          </div>
          <button class="relations-button" :disabled="duplicateRelations.length === 0" @click="openDialog('relations')">
            <span>중복 관계 보기</span>
            <ArrowRight :size="16"  />
          </button>
        </div>
      </section>
    </div>

    <footer class="action-bar">
      <div class="hint-box">
        <Info :size="16"  />
        <span>{{ actionHint }}</span>
      </div>
      <button class="danger-button" :disabled="state.running" @click="openDialog('cacheConfirm')">
        <Trash2 :size="16"  />
        <span>캐시 삭제</span>
      </button>
      <button class="run-button" :class="{ stop: state.running }" :disabled="pending || (!state.running && !canStartScan)" @click="runOrStop">
        <Play v-if="!state.running" :size="16"  />
        <span v-else class="stop-glyph"></span>
        <span>{{ state.running ? '검사 중지' : '검사 시작' }}</span>
      </button>
    </footer>

    <div v-if="state.dialog !== 'none'" class="modal-backdrop" @click.self="closeDialog">
      <section class="modal-card">
        <template v-if="state.dialog === 'cacheConfirm'">
          <h3>SQLite 캐시 삭제</h3>
          <p>SQLite 해시 캐시를 삭제하면 다음 검사에서 필요한 해시를 다시 계산합니다.</p>
          <div class="modal-actions">
            <button class="small-button" @click="closeDialog">취소</button>
            <button class="danger-button compact" @click="confirmClearCache">삭제</button>
          </div>
        </template>

        <template v-else-if="state.dialog === 'scanConfirm'">
          <h3>검사 시작</h3>
          <p>선택한 디렉터리에서 중복 파일 검사를 시작하시겠습니까?</p>
          <div class="confirm-list">
            <span>검사 경로</span>
            <div class="confirm-paths">
              <strong v-for="path in state.paths" :key="path" :title="path">{{ path }}</strong>
            </div>
            <span>검사 모드</span>
            <strong>{{ selectedScanMode.label }}</strong>
            <span>비교 기준</span>
            <strong>{{ selectedAlgorithm.label }}</strong>
          </div>
          <div class="modal-actions">
            <button class="small-button" @click="closeDialog">취소</button>
            <button class="run-button compact" :disabled="pending" @click="confirmStartScan">시작</button>
          </div>
        </template>

        <template v-else-if="state.dialog === 'scanMode'">
          <h3>검사 모드</h3>
          <div class="option-list">
            <button v-for="mode in scanModes" :key="mode.id" class="option-row" :class="{ selected: state.scanMode === mode.id }" @click="updateScanMode(mode.id)">
              <strong>{{ mode.label }}</strong>
              <span>{{ mode.description }}</span>
            </button>
          </div>
          <div class="modal-actions"><button class="small-button" @click="closeDialog">닫기</button></div>
        </template>

        <template v-else-if="state.dialog === 'algorithm'">
          <h3>비교 기준</h3>
          <div class="option-list">
            <button v-for="algorithm in algorithms" :key="algorithm.id" class="option-row" :class="{ selected: state.algorithm === algorithm.id }" @click="updateAlgorithm(algorithm.id)">
              <strong>{{ algorithm.label }}</strong>
              <span>{{ algorithm.description }}</span>
            </button>
          </div>
          <div class="modal-actions"><button class="small-button" @click="closeDialog">닫기</button></div>
        </template>

        <template v-else-if="state.dialog === 'paths'">
          <h3>검사 경로</h3>
          <div class="path-list">
            <p v-if="state.paths.length === 0" class="empty-copy">선택된 폴더가 없습니다.</p>
            <label v-for="path in state.paths" :key="path" class="check-row">
              <input type="checkbox" :checked="state.selectedPaths.includes(path)" @change="handlePathSelection(path, $event)" />
              <span>{{ path }}</span>
            </label>
          </div>
          <div class="modal-actions">
            <button class="small-button" @click="closeDialog">닫기</button>
            <button class="danger-button compact" :disabled="state.selectedPaths.length === 0" @click="removeSelectedPaths">선택 삭제</button>
          </div>
        </template>

        <template v-else-if="state.dialog === 'cacheLimit'">
          <h3>SQLite 캐시 제한</h3>
          <label class="unit-input">
            <input v-model="state.cacheLimitInput" class="modal-input" inputmode="numeric" />
            <span>MB</span>
          </label>
          <div class="modal-actions">
            <button class="small-button" @click="closeDialog">취소</button>
            <button class="run-button compact" @click="updateCacheLimit">저장</button>
          </div>
        </template>

        <template v-else-if="state.dialog === 'quarantine'">
          <h3>보관 폴더</h3>
          <div class="quarantine-list">
            <p v-if="state.destinations.length === 0" class="empty-copy">검사 경로를 먼저 선택하십시오.</p>
            <div v-for="destination in state.destinations" :key="destination.volume_key" class="quarantine-row">
              <div>
                <strong v-for="root in destination.root_paths" :key="root">{{ root }}</strong>
                <span>{{ destination.target_path }}</span>
              </div>
              <button class="small-button" :class="{ unconfigured: !destination.configured }" @click="chooseQuarantine(destination)">폴더 선택</button>
            </div>
          </div>
          <div class="modal-actions"><button class="small-button" @click="closeDialog">닫기</button></div>
        </template>

        <template v-else-if="state.dialog === 'relations'">
          <h3>중복 관계</h3>
          <div class="filter-bar">
            <button v-for="filter in relationFilters" :key="filter.id" class="filter-button" :class="{ selected: state.relationFilter === filter.id }" @click="state.relationFilter = filter.id">
              {{ filter.label }} {{ relationFilterCount(filter.id) }}
            </button>
          </div>
          <div class="relation-list">
            <p v-if="filteredRelations.length === 0" class="empty-copy">선택한 필터에 해당하는 중복 관계가 없습니다.</p>
            <article v-for="(relation, index) in filteredRelations" :key="`${relation.current_duplicate_path}-${index}`" class="relation-card">
              <header>
                <strong>관계 {{ index + 1 }}</strong>
                <span>{{ relationKindLabel(relation.kind) }} · {{ formatBytes(relation.size) }} · {{ compactHash(relation.hash) }}</span>
              </header>
              <div class="relation-paths">
                <div>
                  <span>원본 파일</span>
                  <p :title="relation.original_path">{{ relation.original_path }}</p>
                  <button class="small-button" @click="openFile(relation.original_path)">위치 열기</button>
                </div>
                <div>
                  <span>보관 위치</span>
                  <p :title="relation.current_duplicate_path">{{ relation.current_duplicate_path }}</p>
                  <button class="small-button" @click="openFile(relation.current_duplicate_path)">위치 열기</button>
                </div>
              </div>
            </article>
          </div>
          <div class="modal-actions">
            <button class="small-button" @click="saveRelationsLog">로그 저장</button>
            <button class="run-button compact" @click="closeDialog">닫기</button>
          </div>
        </template>
      </section>
    </div>
  </main>
</template>
