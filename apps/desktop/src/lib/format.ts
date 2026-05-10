import type { ActivityEvent, CleanReport, DuplicateRelation } from './native'

export function formatBytes(value: number) {
  const units = ['B', 'KB', 'MB', 'GB', 'TB', 'PB']
  let size = value
  let index = 0

  while (size >= 1024 && index < units.length - 1) {
    size /= 1024
    index += 1
  }

  if (index === 0) {
    return `${Math.round(size)} ${units[index]}`
  }

  return `${size.toFixed(size >= 10 ? 1 : 2)} ${units[index]}`
}

export function compactPath(path: string, max = 22) {
  if (path.length <= max) {
    return path
  }

  const normalized = path.replaceAll('\\', '/')
  const parts = normalized.split('/').filter(Boolean)
  const tail = parts.at(-1) ?? normalized

  if (tail.length + 4 <= max) {
    return `.../${tail}`
  }

  return `${tail.slice(0, Math.max(1, max - 3))}...`
}

export function compactHash(hash: string) {
  return hash.length <= 14 ? hash : `${hash.slice(0, 14)}...`
}

export function progressFromReport(report: CleanReport | null) {
  if (!report) {
    return 0
  }

  const total = report.candidate_files + report.duplicate_groups

  if (total === 0) {
    return report.scanned_files > 0 ? 1 : 0
  }

  return Math.min(1, (report.hashed_files + report.reused_hashes + report.duplicate_groups) / total)
}

export function relationKindLabel(kind: string) {
  return kind === 'SameNameAndSize' || kind === 'SAME_NAME_AND_SIZE'
    ? '같은 이름+용량'
    : '다른 이름+용량+해시'
}

export function formatActivityLog(events: ActivityEvent[]) {
  return events
    .map((event) => {
      const progress = event.progress == null ? '' : `\t${(event.progress * 100).toFixed(1)}%`
      const path = event.path == null ? '' : `\t${event.path}`
      return `${event.stage}\t${event.detail}${progress}${path}`
    })
    .join('\n')
}

export function formatDuplicateRelationsLog(relations: DuplicateRelation[]) {
  return relations
    .map((relation) =>
      [
        relationKindLabel(relation.kind),
        relation.size,
        relation.hash,
        relation.original_path,
        relation.current_duplicate_path
      ].join('\t')
    )
    .join('\n')
}
