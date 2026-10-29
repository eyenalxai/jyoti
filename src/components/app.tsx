import { useRenderer, useTerminalDimensions } from "@opentui/solid"
import { createEffect, createMemo, createSignal, onCleanup } from "solid-js"

import type { ControlFocus } from "@/components/bulb/controls"
import type { Screen } from "@/components/bulb/modals"
import type { Pane } from "@/components/bulb/pane"
import type { Bulb } from "@/lib/bulbs/schema"

import { initialColor } from "@/components/bulb/color/initial"
import { useColorSession } from "@/components/bulb/color/use-session"
import { Dashboard } from "@/components/bulb/dashboard"
import { canFocus } from "@/components/bulb/focus"
import { Modals } from "@/components/bulb/modals"
import { useBulbActions } from "@/components/bulb/use-actions"
import { useAppCommands } from "@/components/bulb/use-app-commands"
import { useBulbs } from "@/components/bulb/use-bulbs"
import { useControls } from "@/components/bulb/use-controls"
import { useForms } from "@/components/bulb/use-forms"
import { useTerminalPalette } from "@/components/bulb/use-terminal-palette"
import { useCapturePreview } from "@/components/screen/use-preview"
import { useScreenSyncModel } from "@/components/screen/use-sync"
import { Toaster } from "@/components/ui/toast"
import { useToasts } from "@/components/ui/use-toasts"
import { aggregateState, lightQualityOf } from "@/lib/bulbs/aggregate"
import { dispose } from "@/lib/runtime"
import { palette } from "@/lib/theme"

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
  const selectedQuality = createMemo(() =>
    model.allSelected()
      ? aggregateState(model.bulbs(), model.states(), model.stateErrors()).quality
      : lightQualityOf(model.selectedState()?.mode),
  )
  // Only the rows the current mode draws can hold focus.
  createEffect(() => {
    if (!canFocus(selectedQuality(), focus())) {
      setFocus("quality")
    }
  })
  const actions = useBulbActions({
    model,
    syncActive,
    busy,
    notify,
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
  const commands = useAppCommands({
    model,
    controls,
    actions,
    sync,
    preview,
    screen,
    focus,
    pane,
    busy,
    syncActive,
    selectedQuality,
    colorInitial,
    colorSession,
    setScreen,
    setActiveBulb,
    setFocus,
    setPane,
    notify,
    quit,
    confirmRemove: forms.confirmRemove,
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
        colorOpen={screen() === "color"}
        colorBusy={busy()}
        colorInitial={colorInitial()}
        onFocus={commands.focusControl}
        onQuality={commands.setQuality}
        onPreviewColor={colorSession.preview}
        onSubmitColor={forms.submitColor}
        onDismissColor={commands.dismissColor}
      />
      <Modals
        screen={screen()}
        activeBulb={activeBulb()}
        busy={busy()}
        preview={preview}
        syncModel={sync}
        onSubmitAdd={forms.submitAdd}
        onSubmitEdit={forms.submitEdit}
        onConfirmRemove={forms.confirmRemove}
      />
      <Toaster toasts={toasts()} onDismiss={dismissToast} />
    </box>
  )
}

export { App }
