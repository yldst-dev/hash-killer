const blockedShortcutKeys = new Set([
  '0',
  '+',
  ',',
  '-',
  '=',
  '_',
  'a',
  'c',
  'f',
  'i',
  'j',
  'l',
  'n',
  'o',
  'p',
  'r',
  's',
  't',
  'u',
  'w'
])

function stop(event: Event) {
  event.preventDefault()
  event.stopPropagation()
}

function handleKeydown(event: KeyboardEvent) {
  const key = event.key.toLowerCase()

  if (event.key === 'F5' || event.key === 'F7' || event.key === 'F11' || event.key === 'F12') {
    stop(event)
    return
  }

  if (event.altKey && (event.key === 'ArrowLeft' || event.key === 'ArrowRight')) {
    stop(event)
    return
  }

  if ((event.metaKey || event.ctrlKey) && blockedShortcutKeys.has(key)) {
    stop(event)
  }
}

export function installWebviewLockdown() {
  window.addEventListener('contextmenu', stop, { capture: true })
  window.addEventListener('auxclick', stop, { capture: true })
  window.addEventListener('selectstart', stop, { capture: true })
  window.addEventListener('dragstart', stop, { capture: true })
  window.addEventListener('drop', stop, { capture: true })
  window.addEventListener('keydown', handleKeydown, { capture: true })

  document.addEventListener('gesturestart', stop, { capture: true })
}
