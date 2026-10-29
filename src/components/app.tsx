import { useRenderer, useTerminalDimensions } from "@opentui/solid"
import { createMemo, createSignal, onCleanup } from "solid-js"

import type { ControlFocus } from "@/components/bulb/controls"
import type { Pane } from "@/components/bulb/dashboard"
import type { Screen } from "@/components/bulb/modals"
import type { Notice } from "@/components/ui/status"
import type { Bulb } from "@/lib/bulbs/schema"
import type { Timer } from "@/lib/timers"

import { Dashboard } from "@/components/bulb/dashboard"
import { Modals } from "@/components/bulb/modals"
import { useAppKeys } from "@/components/bulb/use-app-keys"
import { useBulbs } from "@/components/bulb/use-bulbs"
import { useControls } from "@/components/bulb/use-controls"
import { useForms } from "@/components/bulb/use-forms"
import { useTerminalPalette } from "@/components/bulb/use-terminal-palette"
import { aggregateState } from "@/lib/bulbs/aggregate"
import { BRIGHTNESS_STEP, TEMPERATURE_STEP } from "@/lib/bulbs/schema"
import { noticeDuration } from "@/lib/format"
import { dispose } from "@/lib/runtime"
import { palette } from "@/lib/theme"
import { after, cancel } from "@/lib/timers"

const focusOrder: readonly ControlFocus[] = ["power", "brightness", "temperature", "color"]
const App = () => {
  const renderer = useRenderer()
  const dimensions = useTerminalDimensions()
  useTerminalPalette()
  const [screen, setScreen] = createSignal<Screen>("dashboard")
  const [activeBulb, setActiveBulb] = createSignal<Bulb | null>(null)
  const [formError, setFormError] = createSignal<string | null>(null)
  const [busy, setBusy] = createSignal(false)
  const [focus, setFocus] = createSignal<ControlFocus>("power")
  const [pane, setPane] = createSignal<Pane>("bulbs")
  const [notice, setNotice] = createSignal<Notice | null>(null)
  let noticeTimer: Timer | null = null

  const notify = (next: Notice) => {
    if (noticeTimer !== null) {
      cancel(noticeTimer)
    }
    setNotice(next)
    noticeTimer = after(noticeDuration(next.message), () => {
      setNotice(null)
    })
  }

  const fail = (message: string) => {
    notify({ kind: "error", message })
  }

  const model = useBulbs({ onError: fail })
  const controls = useControls({ model, onError: fail })
  const forms = useForms({
    model,
    controls,
    activeBulb,
    busy,
    setActiveBulb,
    setBusy,
    setFormError,
    setScreen,
    notify,
  })

  const colorInitial = createMemo(() => {
    if (!model.allSelected()) {
      return model.selectedState()?.color ?? "#FFFFFF"
    }
    const aggregate = aggregateState(model.bulbs(), model.states(), model.stateErrors())
    return aggregate.color?.kind === "equal" ? aggregate.color.hex : "#FFFFFF"
  })

  const closeScreen = () => {
    if (busy()) {
      return
    }
    setScreen("dashboard")
    setActiveBulb(null)
    setFormError(null)
  }

  const openAdd = () => {
    if (busy()) {
      return
    }
    setFormError(null)
    setActiveBulb(null)
    setScreen("add")
  }

  const openEdit = () => {
    if (model.allSelected()) {
      notify({ kind: "info", message: "All bulbs is a group. Select a bulb to edit it." })
      return
    }
    const bulb = model.selected()
    if (bulb === null || busy()) {
      return
    }
    setFormError(null)
    setActiveBulb(bulb)
    setScreen("edit")
  }

  const openRemove = () => {
    if (model.allSelected()) {
      notify({ kind: "info", message: "All bulbs is a group. Select a bulb to remove it." })
      return
    }
    const bulb = model.selected()
    if (bulb === null || busy()) {
      return
    }
    setFormError(null)
    setActiveBulb(bulb)
    setScreen("remove")
  }

  const openColor = () => {
    if (busy()) {
      return
    }
    if (!model.allSelected() && model.selected() === null) {
      return
    }
    setFormError(null)
    setActiveBulb(model.selected())
    setScreen("color")
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
      openColor()
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
    onQuit: () => {
      renderer.destroy()
    },
    onClose: closeScreen,
    onAdd: openAdd,
    onEdit: openEdit,
    onRemove: openRemove,
    onColor: openColor,
    onRefresh: model.refresh,
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
  })

  onCleanup(() => {
    if (noticeTimer !== null) {
      cancel(noticeTimer)
    }
    void dispose()
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
        notice={notice()}
        width={dimensions().width}
        height={dimensions().height}
        onFocus={focusControl}
      />
      <Modals
        screen={screen()}
        activeBulb={activeBulb()}
        colorInitial={colorInitial()}
        busy={busy()}
        error={formError()}
        onSubmitAdd={forms.submitAdd}
        onSubmitEdit={forms.submitEdit}
        onConfirmRemove={forms.confirmRemove}
        onSubmitColor={forms.submitColor}
      />
    </box>
  )
}

export { App }
