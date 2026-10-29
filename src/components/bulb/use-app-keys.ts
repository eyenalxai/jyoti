import { useKeyboard } from "@opentui/solid"

import type { ControlFocus } from "@/components/bulb/controls"
import type { Screen } from "@/components/bulb/modals"

interface AppKeysOptions {
  readonly screen: () => Screen
  readonly onQuit: () => void
  readonly onClose: () => void
  readonly onAdd: () => void
  readonly onEdit: () => void
  readonly onRemove: () => void
  readonly onColor: () => void
  readonly onRefresh: () => void
  readonly onSyncToggle: () => void
  readonly onSyncPause: () => void
  readonly onMonitor: () => void
  readonly onHelp: () => void
  readonly onTogglePower: () => void
  readonly onSelectAll: () => void
  readonly onTogglePane: () => void
  readonly onWhite: () => void
  readonly onFocus: (focus: ControlFocus) => void
  readonly onMove: (delta: number) => void
  readonly onAdjust: (direction: number) => void
  readonly onActivate: () => void
  readonly onConfirmRemove: () => void
  readonly onPreview: () => void
  readonly onPreviewMove: (delta: number) => void
  readonly onPreviewToggle: () => void
  readonly onPreviewClose: () => void
  readonly onMonitorMove: (delta: number) => void
  readonly onMonitorChoose: () => void
}

interface KeyLike {
  readonly name: string
  readonly shift: boolean
  readonly ctrl: boolean
  readonly eventType: string
}

const isQuestionMark = (key: KeyLike): boolean =>
  key.name === "?" || (key.shift && key.name === "/")

const isSelectAll = (key: KeyLike): boolean =>
  !key.ctrl && (key.name === "A" || (key.shift && key.name === "a"))

const isUp = (key: KeyLike): boolean => key.name === "up" || key.name === "k"
const isDown = (key: KeyLike): boolean => key.name === "down" || key.name === "j"
const isEnter = (key: KeyLike): boolean => key.name === "enter" || key.name === "return"
const isEscape = (key: KeyLike): boolean => key.name === "escape"

const handlePreview = (key: KeyLike, options: AppKeysOptions): void => {
  if (isEscape(key) || key.name === "v") {
    options.onPreviewClose()
    return
  }
  if (key.name === "q") {
    options.onQuit()
    return
  }
  if (isUp(key)) {
    options.onPreviewMove(-1)
    return
  }
  if (isDown(key)) {
    options.onPreviewMove(1)
    return
  }
  if (isEnter(key)) {
    options.onPreviewToggle()
  }
}

const handleMonitors = (key: KeyLike, options: AppKeysOptions): void => {
  if (isEscape(key)) {
    options.onClose()
    return
  }
  if (isUp(key)) {
    options.onMonitorMove(-1)
    return
  }
  if (isDown(key)) {
    options.onMonitorMove(1)
    return
  }
  if (isEnter(key)) {
    options.onMonitorChoose()
  }
}

const handleHelp = (key: KeyLike, options: AppKeysOptions): void => {
  if (isEscape(key) || isQuestionMark(key)) {
    options.onClose()
  }
  if (key.name === "q") {
    options.onQuit()
  }
}

const handleRemove = (key: KeyLike, options: AppKeysOptions): void => {
  if (key.name === "y") {
    options.onConfirmRemove()
  }
  if (key.name === "n" || isEscape(key)) {
    options.onClose()
  }
}

const handleModal = (key: KeyLike, screen: Screen, options: AppKeysOptions): void => {
  if (screen === "capture") {
    handlePreview(key, options)
    return
  }
  if (screen === "monitors") {
    handleMonitors(key, options)
    return
  }
  if (screen === "help") {
    handleHelp(key, options)
    return
  }
  if (screen === "remove") {
    handleRemove(key, options)
    return
  }
  if (isEscape(key)) {
    options.onClose()
  }
}

const handleSelection = (key: KeyLike, options: AppKeysOptions): boolean => {
  if (isUp(key)) {
    options.onMove(-1)
    return true
  }
  if (isDown(key)) {
    options.onMove(1)
    return true
  }
  if (key.name === "tab") {
    options.onTogglePane()
    return true
  }
  if (key.name === "b") {
    options.onFocus("brightness")
    return true
  }
  if (key.name === "t") {
    options.onFocus("light")
    return true
  }
  if (key.name === "left" || key.name === "h") {
    options.onAdjust(-1)
    return true
  }
  if (key.name === "right" || key.name === "l") {
    options.onAdjust(1)
    return true
  }
  return false
}

const handleAction = (key: KeyLike, options: AppKeysOptions): boolean => {
  if (key.name === "q") {
    options.onQuit()
    return true
  }
  if (isSelectAll(key)) {
    options.onSelectAll()
    return true
  }
  if (key.name === "a") {
    options.onAdd()
    return true
  }
  if (key.name === "e") {
    options.onEdit()
    return true
  }
  if (key.name === "d") {
    options.onRemove()
    return true
  }
  if (key.name === "c") {
    options.onColor()
    return true
  }
  if (key.name === "w") {
    options.onWhite()
    return true
  }
  if (key.name === "r") {
    options.onRefresh()
    return true
  }
  if (key.name === "s") {
    options.onSyncToggle()
    return true
  }
  if (key.name === "x") {
    options.onSyncPause()
    return true
  }
  if (key.name === "m") {
    options.onMonitor()
    return true
  }
  if (key.name === "v") {
    options.onPreview()
    return true
  }
  if (isQuestionMark(key)) {
    options.onHelp()
    return true
  }
  if (key.name === "space" || key.name === "p") {
    options.onTogglePower()
    return true
  }
  if (isEnter(key)) {
    options.onActivate()
    return true
  }
  return false
}

const handleDashboard = (key: KeyLike, options: AppKeysOptions): void => {
  if (handleAction(key, options)) {
    return
  }
  handleSelection(key, options)
}

const useAppKeys = (options: AppKeysOptions): void => {
  useKeyboard((key) => {
    if (key.eventType === "release") {
      return
    }
    if (key.ctrl && key.name === "c") {
      options.onQuit()
      return
    }
    const screen = options.screen()
    if (screen === "dashboard") {
      handleDashboard(key, options)
      return
    }
    handleModal(key, screen, options)
  })
}

export { useAppKeys }
