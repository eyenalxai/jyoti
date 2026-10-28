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
  readonly onHelp: () => void
  readonly onTogglePower: () => void
  readonly onTogglePane: () => void
  readonly onFocus: (focus: ControlFocus) => void
  readonly onMove: (delta: number) => void
  readonly onAdjust: (direction: number) => void
  readonly onActivate: () => void
  readonly onConfirmRemove: () => void
}

interface KeyLike {
  readonly name: string
  readonly shift: boolean
  readonly ctrl: boolean
  readonly eventType: string
}

const isQuestionMark = (key: KeyLike): boolean =>
  key.name === "?" || (key.shift && key.name === "/")

const handleModal = (key: KeyLike, screen: Screen, options: AppKeysOptions): void => {
  if (screen === "help") {
    if (key.name === "escape" || isQuestionMark(key)) {
      options.onClose()
    }
    if (key.name === "q") {
      options.onQuit()
    }
    return
  }
  if (screen === "remove") {
    if (key.name === "y") {
      options.onConfirmRemove()
    }
    if (key.name === "n" || key.name === "escape") {
      options.onClose()
    }
    return
  }
  if (key.name === "escape") {
    options.onClose()
  }
}

const handleSelection = (key: KeyLike, options: AppKeysOptions): boolean => {
  if (key.name === "up" || key.name === "k") {
    options.onMove(-1)
    return true
  }
  if (key.name === "down" || key.name === "j") {
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
    options.onFocus("temperature")
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
  if (key.name === "r") {
    options.onRefresh()
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
  if (key.name === "enter" || key.name === "return") {
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
