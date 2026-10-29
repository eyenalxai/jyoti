import { useRenderer, useTerminalDimensions } from "@opentui/solid"
import { createMemo, createSignal, onCleanup } from "solid-js"

import type { ControlFocus } from "@/components/bulb/controls"
import type { Pane } from "@/components/bulb/dashboard"
import type { Screen } from "@/components/bulb/modals"
import type { Bulb } from "@/lib/bulbs/schema"

import { initialColor } from "@/components/bulb/color/initial"
import { useColorSession } from "@/components/bulb/color/use-session"
import { Dashboard } from "@/components/bulb/dashboard"
import { Modals } from "@/components/bulb/modals"
import { useBulbActions } from "@/components/bulb/use-actions"
import { useAppKeys } from "@/components/bulb/use-app-keys"
import { useBulbs } from "@/components/bulb/use-bulbs"
import { useControls } from "@/components/bulb/use-controls"
import { useForms } from "@/components/bulb/use-forms"
import { useTerminalPalette } from "@/components/bulb/use-terminal-palette"
import { useCapturePreview } from "@/components/screen/use-preview"
import { useScreenSyncModel } from "@/components/screen/use-sync"
import { Toaster } from "@/components/ui/toast"
import { useToasts } from "@/components/ui/use-toasts"
import { BRIGHTNESS_STEP, TEMPERATURE_STEP } from "@/lib/bulbs/schema"
import { dispose } from "@/lib/runtime"
import { SYNC_OWNS_CONTROLS } from "@/lib/screen/sync/copy"
import { palette } from "@/lib/theme"

const focusOrder: readonly ControlFocus[] = ["power", "brightness", "temperature", "color"]
const App = () => {
  const renderer = useRenderer()
  const dimensions = useTerminalDimensions()
  useTerminalPalette()
  const [screen, setScreen] = createSignal<Screen>("dashboard")
  const [activeBulb, setActiveBulb] = createSignal<Bulb | null>(null)
  const [busy, setBusy] = createSignal(false)
  const [focus, setFocus] = createSignal<ControlFocus>("power")
  const [pane, setPane] = createSignal<Pane>("bulbs")
  const { toasts, notify, dismiss: dismissToast } = useToasts()
  let quitting = false

  const fail = (message: string) => {
    notify({ kind: "error", message })
  }

  const preview = useCapturePreview({ onError: fail })
  const sync = useScreenSyncModel({ onNotice: notify, onError: fail })
  const model = useBulbs({ onError: fail, onPower: sync.notePowers })
  const syncActive = createMemo(() => sync.status().phase !== "idle")
  const controls = useControls({
    model,
    onError: fail,
    syncLocked: syncActive,
    onBlocked: (message) => {
      notify({ kind: "info", message })
    },
  })
  const colorSession = useColorSession({ model, controls, notify })
  const quit = () => {
    if (quitting) {
      return
    }
    quitting = true
    void sync.stop().finally(() => {
      renderer.destroy()
    })
  }
  const syncOwnsControls = () =>
    syncActive() && (focus() === "brightness" || focus() === "temperature")
  const actions = useBulbActions({
    model,
    syncActive,
    busy,
    notify,
    captureColor: colorSession.capture,
    setActiveBulb,
    setScreen,
  })
  const forms = useForms({
    model,
    sendColor: colorSession.commit,
    activeBulb,
    busy,
    setActiveBulb,
    setBusy,
    setScreen,
    notify,
  })
  const colorInitial = createMemo(() => initialColor(model))

  const closeColor = (revert: boolean) => {
    if (revert && colorSession.dirty()) {
      colorSession.revert()
    }
    setScreen("dashboard")
    setActiveBulb(null)
    colorSession.reset()
  }

  const closeScreen = () => {
    if (busy()) {
      return
    }
    if (screen() === "color") {
      closeColor(true)
      return
    }
    setScreen("dashboard")
    setActiveBulb(null)
  }

  const dismissColor = () => {
    closeColor(false)
  }

  const selectAll = () => {
    if (!model.canSelectAll()) {
      notify({
        kind: "info",
        message:
          model.bulbs().length === 0
            ? "Add a bulb first."
            : "Add a second bulb to control all of them.",
      })
      return
    }
    model.selectAll()
    setFocus("power")
  }

  const cycleFocus = (direction: number) => {
    const current = focusOrder.indexOf(focus())
    const base = current === -1 ? 0 : current
    const next = focusOrder[(base + direction + focusOrder.length) % focusOrder.length]
    if (next !== undefined) {
      setFocus(next)
    }
  }
  const move = (delta: number) => {
    if (pane() === "bulbs") {
      model.move(delta)
      return
    }
    cycleFocus(delta)
  }
  const adjust = (direction: number) => {
    if (pane() !== "controls") {
      return
    }
    if (syncOwnsControls()) {
      notify({ kind: "info", message: SYNC_OWNS_CONTROLS })
      return
    }
    if (focus() === "brightness") {
      controls.adjustBrightness(direction * BRIGHTNESS_STEP)
    }
    if (focus() === "temperature") {
      controls.adjustTemperature(direction * TEMPERATURE_STEP)
    }
  }
  const activate = () => {
    if (pane() === "bulbs") {
      setPane("controls")
      return
    }
    if (focus() === "power") {
      void controls.togglePower()
      return
    }
    if (focus() === "color") {
      actions.openColor()
      return
    }
    if (syncOwnsControls()) {
      notify({ kind: "info", message: SYNC_OWNS_CONTROLS })
      return
    }
    controls.flushPending()
  }

  const togglePane = () => {
    setPane((current) => (current === "bulbs" ? "controls" : "bulbs"))
  }

  const focusControl = (next: ControlFocus) => {
    setPane("controls")
    setFocus(next)
  }

  useAppKeys({
    screen: () => screen(),
    onQuit: quit,
    onClose: closeScreen,
    onAdd: actions.openAdd,
    onEdit: actions.openEdit,
    onRemove: actions.openRemove,
    onColor: actions.openColor,
    onRefresh: () => {
      model.refresh()
      sync.clearRestore()
    },
    onSyncToggle: sync.toggle,
    onSyncPause: sync.togglePause,
    onMonitor: () => {
      if (sync.status().phase === "stopping") {
        notify({ kind: "info", message: "Wait for sync to stop." })
        return
      }
      sync.openMonitors()
      setScreen("monitors")
    },
    onHelp: () => {
      setScreen("help")
    },
    onTogglePower: () => {
      void controls.togglePower()
    },
    onSelectAll: selectAll,
    onTogglePane: togglePane,
    onFocus: focusControl,
    onMove: move,
    onAdjust: adjust,
    onActivate: activate,
    onConfirmRemove: forms.confirmRemove,
    onPreview: () => {
      if (syncActive()) {
        notify({
          kind: "info",
          message: "Stop screen sync before opening the capture preview.",
        })
        return
      }
      preview.open()
      setScreen("capture")
    },
    onPreviewMove: preview.move,
    onPreviewToggle: preview.toggle,
    onPreviewClose: () => {
      preview.close()
      setScreen("dashboard")
    },
    onMonitorMove: sync.moveMonitor,
    onMonitorChoose: () => {
      void sync.chooseMonitor().then((chosen) => {
        if (chosen) {
          setScreen("dashboard")
        }
        return null
      })
    },
  })

  onCleanup(() => {
    void sync.stop().finally(() => {
      void dispose()
    })
  })

  return (
    <box
      width="100%"
      height="100%"
      flexDirection="column"
      backgroundColor={palette.background}
      paddingX={1}
      position="relative"
    >
      <Dashboard
        model={model}
        focus={focus()}
        pane={pane()}
        syncStatus={sync.status()}
        width={dimensions().width}
        height={dimensions().height}
        onFocus={focusControl}
      />
      <Modals
        screen={screen()}
        activeBulb={activeBulb()}
        colorInitial={colorInitial()}
        busy={busy()}
        preview={preview}
        syncModel={sync}
        onSubmitAdd={forms.submitAdd}
        onSubmitEdit={forms.submitEdit}
        onConfirmRemove={forms.confirmRemove}
        onSubmitColor={forms.submitColor}
        onPreviewColor={colorSession.preview}
        onDismissColor={dismissColor}
      />
      <Toaster toasts={toasts()} onDismiss={dismissToast} />
    </box>
  )
}

export { App }
