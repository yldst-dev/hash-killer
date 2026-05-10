import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

export type HashAlgorithmId = 'BLAKE3' | 'SHA256' | 'SHA512' | 'MD5'
export type ScanModeId = 'FAST' | 'FULL_HASH' | 'REHASH'

export interface ActivityEvent {
  stage: string
  detail: string
  path: string | null
  progress: number | null
  completed: number | null
  total: number | null
}

export interface DuplicateRelation {
  original_path: string
  duplicate_path: string
  current_duplicate_path: string
  size: number
  hash: string
  kind: string
}

export interface CleanReport {
  scanned_files: number
  candidate_files: number
  hashed_files: number
  reused_hashes: number
  duplicate_groups: number
  deleted_files: number
  kept_files: number
  reclaimed_bytes: number
  failed_files: string[]
  duplicate_relations: DuplicateRelation[]
}

export interface Settings {
  cache_limit_mb: number
  cache_limit_configured: boolean
  algorithm: HashAlgorithmId
  algorithm_configured: boolean
  scan_mode: ScanModeId
  scan_mode_configured: boolean
}

export interface VolumeDestination {
  volume_key: string
  root_path: string
  root_paths: string[]
  target_path: string
  configured: boolean
}

export interface ScanRun {
  scan_id: number
}

export interface UnitResult {
  ok: boolean
}

export interface ScanActivityPayload {
  scan_id: number
  event: ActivityEvent
}

export interface ScanCompletedPayload {
  scan_id: number
  report: CleanReport
}

export interface ScanFailedPayload {
  scan_id: number
  message: string
}

export interface ScanCancelledPayload {
  scan_id: number
}

function tauriReady() {
  const tauriWindow = window as unknown as { __TAURI_INTERNALS__?: { transformCallback?: unknown } }
  return typeof tauriWindow.__TAURI_INTERNALS__?.transformCallback === 'function'
}

export function startScan(roots: string[], algorithm: HashAlgorithmId, scanMode: ScanModeId) {
  if (!tauriReady()) {
    return Promise.resolve({ scan_id: Date.now() })
  }

  return invoke<ScanRun>('start_scan', {
    request: {
      roots,
      algorithm,
      scan_mode: scanMode
    }
  })
}

export function cancelScan(scanId: number) {
  if (!tauriReady()) {
    return Promise.resolve({ ok: true })
  }

  return invoke<UnitResult>('cancel_scan', { request: { scan_id: scanId } })
}

export function loadSettings() {
  if (!tauriReady()) {
    return Promise.resolve<Settings>({
      cache_limit_mb: 256,
      cache_limit_configured: false,
      algorithm: 'BLAKE3',
      algorithm_configured: false,
      scan_mode: 'FAST',
      scan_mode_configured: false
    })
  }

  return invoke<Settings>('load_settings')
}

export function saveCacheLimit(value: number) {
  if (!tauriReady()) {
    return Promise.resolve({ pruned: 0 })
  }

  return invoke<{ pruned: number }>('save_cache_limit', { request: { value } })
}

export function clearCache() {
  if (!tauriReady()) {
    return Promise.resolve({ removed: 0 })
  }

  return invoke<{ removed: number }>('clear_cache')
}

export function saveHashAlgorithm(value: HashAlgorithmId) {
  if (!tauriReady()) {
    return Promise.resolve({ ok: true })
  }

  return invoke<UnitResult>('save_hash_algorithm', { request: { value } })
}

export function saveScanMode(value: ScanModeId) {
  if (!tauriReady()) {
    return Promise.resolve({ ok: true })
  }

  return invoke<UnitResult>('save_scan_mode', { request: { value } })
}

export function volumeDestinations(roots: string[]) {
  if (!tauriReady()) {
    return Promise.resolve<VolumeDestination[]>(
      roots.map((root) => ({
        volume_key: root,
        root_path: root,
        root_paths: [root],
        target_path: '지정되지 않음',
        configured: false
      }))
    )
  }

  return invoke<VolumeDestination[]>('volume_destinations', { request: { roots } })
}

export function saveQuarantineDestination(volumeKey: string, targetPath: string) {
  if (!tauriReady()) {
    return Promise.resolve({ ok: true })
  }

  return invoke<UnitResult>('save_quarantine_destination', {
    request: {
      volume_key: volumeKey,
      target_path: targetPath
    }
  })
}

export function clearQuarantineDestinations() {
  if (!tauriReady()) {
    return Promise.resolve({ count: 0 })
  }

  return invoke<{ count: number }>('clear_quarantine_destinations')
}

export function revealFile(path: string) {
  if (!tauriReady()) {
    return Promise.resolve({ ok: true })
  }

  return invoke<UnitResult>('reveal_file', { request: { path } })
}

export function pickFolders() {
  if (!tauriReady()) {
    return Promise.resolve<string[]>([])
  }

  return invoke<string[]>('pick_folders')
}

export function pickFolder(initialDirectory?: string) {
  if (!tauriReady()) {
    return Promise.resolve<string | null>(null)
  }

  return invoke<string | null>('pick_folder', {
    request: { initial_directory: initialDirectory ?? null }
  })
}

export function saveTextFile(suggestedName: string, contents: string) {
  if (!tauriReady()) {
    return Promise.resolve({ ok: false })
  }

  return invoke<UnitResult>('save_text_file', {
    request: {
      suggested_name: suggestedName,
      contents
    }
  })
}

export function onScanActivity(callback: (payload: ScanActivityPayload) => void) {
  if (!tauriReady()) {
    return Promise.resolve(() => {})
  }

  return listen<ScanActivityPayload>('scan://activity', (event) => callback(event.payload))
}

export function onScanCompleted(callback: (payload: ScanCompletedPayload) => void) {
  if (!tauriReady()) {
    return Promise.resolve(() => {})
  }

  return listen<ScanCompletedPayload>('scan://completed', (event) => callback(event.payload))
}

export function onScanFailed(callback: (payload: ScanFailedPayload) => void) {
  if (!tauriReady()) {
    return Promise.resolve(() => {})
  }

  return listen<ScanFailedPayload>('scan://failed', (event) => callback(event.payload))
}

export function onScanCancelled(callback: (payload: ScanCancelledPayload) => void) {
  if (!tauriReady()) {
    return Promise.resolve(() => {})
  }

  return listen<ScanCancelledPayload>('scan://cancelled', (event) => callback(event.payload))
}
