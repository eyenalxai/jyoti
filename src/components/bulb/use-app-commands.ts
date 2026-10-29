import type { Accessor, Setter } from "solid-js"

import type { ColorSessionModel } from "@/components/bulb/color/use-session"
import type { ControlFocus } from "@/components/bulb/controls"
import type { Screen } from "@/components/bulb/modals"
import type { Pane } from "@/components/bulb/pane"
import type { BulbActions } from "@/components/bulb/use-actions"
import type { BulbsModel } from "@/components/bulb/use-bulbs"
import type { ControlsModel } from "@/components/bulb/use-controls"
import type { SyncRow } from "@/components/screen/rows"
import type { CapturePreviewModel } from "@/components/screen/use-preview"
import type { ScreenSyncModel } from "@/components/screen/use-sync"
import type { Notice } from "@/components/ui/toast"
import type { LightQuality, LightSide } from "@/lib/bulbs/aggregate"
import type { Bulb } from "@/lib/bulbs/schema"

import { canFocus, focusOrder, refusalFor } from "@/components/bulb/focus"
import { useAppKeys } from "@/components/bulb/use-app-keys"
import { useSyncPanel } from "@/components/bulb/use-sync-panel"
import { BRIGHTNESS_STEP, TEMPERATURE_STEP } from "@/lib/bulbs/schema"
import { SYNC_OWNS_CONTROLS } from "@/lib/screen/sync/copy"

interface UseAppCommandsOptions {
  readonly model: BulbsModel
  readonly controls: ControlsModel
  readonly actions: BulbActions
  readonly sync: ScreenSyncModel
  readonly preview: CapturePreviewModel
  readonly screen: Accessor<Screen>
  readonly focus: Accessor<ControlFocus>
  readonly pane: Accessor<Pane>
  readonly busy: Accessor<boolean>
  readonly syncActive: Accessor<boolean>
  readonly selectedQuality: Accessor<LightQuality>
  readonly colorInitial: Accessor<string>
  readonly colorSession: ColorSessionModel
  readonly setScreen: Setter<Screen>
  readonly setActiveBulb: Setter<Bulb | null>
  readonly setFocus: Setter<ControlFocus>
  readonly setPane: Setter<Pane>
  readonly notify: (notice: Notice) => void
  readonly quit: () => void
  readonly confirmRemove: () => void
}

interface AppCommands {
  readonly focusControl: (focus: ControlFocus) => void
  readonly setQuality: (quality: LightSide) => void
  readonly dismissColor: () => void
  readonly syncRow: Accessor<SyncRow>
}

const useAppCommands = (options: UseAppCommandsOptions): AppCommands => {
  const syncOwnsControls = () =>
    options.syncActive() &&
    (options.focus() === "quality" ||
      options.focus() === "brightness" ||
      options.focus() === "light")

  const syncPanel = useSyncPanel({ sync: options.sync, setScreen: options.setScreen })

  const closeScreen = () => {
    if (options.busy()) {
      return
    }
    if (options.screen() === "color") {
      if (options.colorSession.dirty()) {
        options.colorSession.revert()
      }
      options.colorSession.reset()
    }
    options.setScreen("dashboard")
    options.setActiveBulb(null)
  }

  const selectAll = () => {
    if (!options.model.canSelectAll()) {
      options.notify({
        kind: "info",
        message:
          options.model.bulbs().length === 0
            ? "Add a bulb first."
            : "Add a second bulb to control all of them.",
      })
      return
    }
    options.model.selectAll()
    options.setFocus("power")
  }

  const cycleFocus = (direction: number) => {
    const order = focusOrder(options.selectedQuality())
    const current = order.indexOf(options.focus())
    const base = current === -1 ? 0 : current
    const next = order[(base + direction + order.length) % order.length]
    if (next !== undefined) {
      options.setFocus(next)
    }
  }

  const move = (delta: number) => {
    if (options.pane() === "bulbs") {
      options.model.move(delta)
      return
    }
    cycleFocus(delta)
  }

  const adjust = (direction: number) => {
    if (options.pane() !== "controls") {
      return
    }
    if (syncOwnsControls()) {
      options.notify({ kind: "info", message: SYNC_OWNS_CONTROLS })
      return
    }
    if (options.focus() === "brightness") {
      options.controls.adjustBrightness(direction * BRIGHTNESS_STEP)
    }
    if (options.focus() === "light" && options.selectedQuality() === "white") {
      options.controls.adjustTemperature(direction * TEMPERATURE_STEP)
    }
  }

  const switchToColor = (seed: boolean) => {
    if (options.syncActive()) {
      options.notify({ kind: "info", message: SYNC_OWNS_CONTROLS })
      return false
    }
    if (!options.colorSession.dirty()) {
      options.colorSession.capture()
    }
    // Entering Colour writes a seed even when an earlier visit left the session dirty.
    if (seed) {
      void options.colorSession.commit(options.colorInitial())
    }
    return true
  }

  const setQuality = (quality: LightSide) => {
    options.setPane("controls")
    options.setFocus("quality")
    if (quality === "white") {
      options.controls.setWhite()
      return
    }
    switchToColor(options.selectedQuality() !== "color")
  }

  const adjustLight = (direction: number) => {
    if (options.pane() !== "controls") {
      return
    }
    if (syncOwnsControls()) {
      options.notify({ kind: "info", message: SYNC_OWNS_CONTROLS })
      return
    }
    if (options.focus() === "quality") {
      const next = direction < 0 ? "white" : "color"
      if (next !== options.selectedQuality()) {
        setQuality(next)
      }
      return
    }
    adjust(direction)
  }

  const openColor = () => {
    if (!switchToColor(options.selectedQuality() === "white")) {
      return
    }
    options.actions.openColor()
  }

  const dismissColor = () => {
    options.colorSession.reset()
    options.setScreen("dashboard")
    options.setActiveBulb(null)
  }

  const activate = () => {
    if (options.pane() === "bulbs") {
      options.setPane("controls")
      return
    }
    if (options.focus() === "power") {
      void options.controls.togglePower()
      return
    }
    if (options.focus() === "quality") {
      const next = options.selectedQuality() === "white" ? "color" : "white"
      setQuality(next)
      return
    }
    if (options.focus() === "light") {
      if (options.selectedQuality() === "color") {
        openColor()
        return
      }
      if (options.selectedQuality() === "white") {
        options.controls.flushPending()
        return
      }
      return
    }
    if (syncOwnsControls()) {
      options.notify({ kind: "info", message: SYNC_OWNS_CONTROLS })
      return
    }
    options.controls.flushPending()
  }

  const togglePane = () => {
    options.setPane((current) => (current === "bulbs" ? "controls" : "bulbs"))
  }

  const focusControl = (next: ControlFocus) => {
    if (!canFocus(options.selectedQuality(), next)) {
      options.notify({ kind: "info", message: refusalFor(next) })
      return
    }
    options.setPane("controls")
    options.setFocus(next)
  }

  const setWhite = () => {
    setQuality("white")
  }

  useAppKeys({
    screen: options.screen,
    onQuit: options.quit,
    onClose: closeScreen,
    onAdd: options.actions.openAdd,
    onEdit: options.actions.openEdit,
    onRemove: options.actions.openRemove,
    onColor: openColor,
    onRefresh: () => {
      options.model.refresh()
      options.sync.clearRestore()
    },
    onSyncPanel: syncPanel.open,
    onSyncPanelMove: syncPanel.move,
    onSyncPanelAdjust: syncPanel.adjust,
    onSyncPanelActivate: syncPanel.activate,
    onSyncPanelClose: syncPanel.close,
    onHelp: () => {
      options.setScreen("help")
    },
    onTogglePower: () => {
      void options.controls.togglePower()
    },
    onSelectAll: selectAll,
    onTogglePane: togglePane,
    onWhite: setWhite,
    onFocus: focusControl,
    onMove: move,
    onAdjust: adjustLight,
    onActivate: activate,
    onConfirmRemove: options.confirmRemove,
    onPreview: () => {
      if (options.syncActive()) {
        options.notify({
          kind: "info",
          message: "Stop screen sync before opening the capture preview.",
        })
        return
      }
      options.preview.open()
      options.setScreen("capture")
    },
    onPreviewMove: options.preview.move,
    onPreviewToggle: options.preview.toggle,
    onPreviewClose: () => {
      options.preview.close()
      options.setScreen("dashboard")
    },
    onMonitorMove: options.sync.moveMonitor,
    onMonitorChoose: () => {
      void options.sync.chooseMonitor().then((chosen) => {
        if (chosen) {
          options.setScreen("dashboard")
        }
        return null
      })
    },
  })

  return { focusControl, setQuality, dismissColor, syncRow: syncPanel.syncRow }
}

export { useAppCommands, type AppCommands }
