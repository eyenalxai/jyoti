import { useKeyboard } from "@opentui/solid"
import { Effect } from "effect"
import { createMemo, createSignal, onCleanup, onMount } from "solid-js"

import type { Mode } from "@/components/bulb/color/mode"
import type { PickerFocus } from "@/components/bulb/color/sliders"
import type { Hint } from "@/components/ui/key"
import type { Rgb } from "@/lib/bulbs/schema"
import type { Timer } from "@/lib/timers"

import { ModeTabs } from "@/components/bulb/color/mode"
import { ColorSliders } from "@/components/bulb/color/sliders"
import { Swatch } from "@/components/bulb/color/swatch"
import { KeyBar } from "@/components/ui/key"
import { formatColor, packColor, parseColor } from "@/lib/bulbs/schema"
import { HUE_MAX, PERCENT_MAX, hsbToRgb, rgbToHsb, withHue, withSaturation } from "@/lib/color/hsb"
import { clamp } from "@/lib/format"
import { forkEffect } from "@/lib/runtime"
import { palette } from "@/lib/theme"
import { after, cancel } from "@/lib/timers"

interface ColorProps {
  readonly initial: string
  readonly busy: boolean
  readonly width: number
  readonly onPreview: (hex: string) => Promise<boolean>
  readonly onSubmit: (hex: string) => void
  readonly onDismiss: () => void
}

const PREVIEW_DELAY_MS = 180
const WHITE: Rgb = { red: 255, green: 255, blue: 255 }

const Color = (props: ColorProps) => {
  const seeded = parseColor(props.initial) ?? WHITE
  const seededHex = formatColor(packColor(seeded))
  const [rgb, setRgb] = createSignal<Rgb>(seeded)
  const [mode, setMode] = createSignal<Mode>("hs")
  const [focus, setFocus] = createSignal<PickerFocus>(0)
  const [hexDraft, setHexDraft] = createSignal(seededHex)
  const [appliedHex, setAppliedHex] = createSignal(seededHex)
  const [issue, setIssue] = createSignal<string | null>(null)
  const [ready, setReady] = createSignal(false)
  let previewTimer: Timer | null = null
  let generation = 0

  onMount(() => {
    const timer = after(10, () => {
      setReady(true)
    })
    onCleanup(() => {
      cancel(timer)
    })
  })

  onCleanup(() => {
    if (previewTimer !== null) {
      cancel(previewTimer)
    }
  })

  const hsb = createMemo(() => rgbToHsb(rgb()))
  const hex = () => formatColor(packColor(rgb()))
  const hexFocus = (): PickerFocus => (mode() === "hs" ? 2 : 3)
  const order = (): readonly PickerFocus[] => (mode() === "hs" ? [0, 1, 2] : [0, 1, 2, 3])
  const meterWidth = (): number => Math.max(10, Math.min(28, props.width - 30))

  const sendPreview = Effect.fn("Color.sendPreview")(function* sendPreview(next: string) {
    generation += 1
    const current = generation
    const applied = yield* Effect.promise(() => props.onPreview(next))
    if (applied && current === generation && hex() === next) {
      setAppliedHex(next)
    }
  })

  const schedulePreview = () => {
    if (previewTimer !== null) {
      cancel(previewTimer)
    }
    previewTimer = after(PREVIEW_DELAY_MS, () => {
      previewTimer = null
      forkEffect(sendPreview(hex()).pipe(Effect.ignoreCause))
    })
  }

  const applyRgb = (next: Rgb, preview = true) => {
    setRgb(next)
    setHexDraft(formatColor(packColor(next)))
    setIssue(null)
    if (preview) {
      schedulePreview()
    }
  }

  const adjust = (direction: number) => {
    if (mode() === "hs") {
      const current = hsb()
      if (focus() === 0) {
        const base =
          current.saturation === 0 ? hsbToRgb({ ...current, saturation: PERCENT_MAX }) : rgb()
        applyRgb(withHue(base, clamp(current.hue + direction, 0, HUE_MAX)))
      }
      if (focus() === 1) {
        applyRgb(withSaturation(rgb(), clamp(current.saturation + direction, 0, PERCENT_MAX)))
      }
      return
    }

    const current = rgb()
    if (focus() === 0) {
      applyRgb({ ...current, red: clamp(current.red + direction, 0, 255) })
    }
    if (focus() === 1) {
      applyRgb({ ...current, green: clamp(current.green + direction, 0, 255) })
    }
    if (focus() === 2) {
      applyRgb({ ...current, blue: clamp(current.blue + direction, 0, 255) })
    }
  }

  const moveFocus = (direction: number) => {
    const positions = order()
    const current = positions.indexOf(focus())
    const next = positions[(current + direction + positions.length) % positions.length]
    if (next === undefined) {
      return
    }
    if (next === hexFocus()) {
      setHexDraft(hex())
    }
    setFocus(next)
  }

  const switchMode = () => {
    setMode((current) => (current === "hs" ? "rgb" : "hs"))
    setFocus(0)
    setHexDraft(hex())
    setIssue(null)
  }

  const editHex = (next: string) => {
    setHexDraft(next)
    // The input emits `input` when its value prop changes, so ignore the canonical echo.
    if (next === hex()) {
      return
    }
    const parsed = parseColor(next)
    if (parsed !== null) {
      applyRgb(parsed)
    }
  }

  const submit = () => {
    if (props.busy) {
      return
    }
    if (previewTimer !== null) {
      cancel(previewTimer)
      previewTimer = null
    }
    const parsed = parseColor(hexDraft())
    if (parsed === null) {
      setIssue("Enter six hexadecimal digits, for example #FF8800.")
      return
    }
    setIssue(null)
    const next = formatColor(packColor(parsed))
    if (next === appliedHex()) {
      props.onDismiss()
      return
    }
    setRgb(parsed)
    props.onSubmit(next)
  }

  useKeyboard((key) => {
    if (key.eventType === "release") {
      return
    }
    if (key.name === "tab") {
      switchMode()
      return
    }
    if (key.name === "up" || key.name === "k") {
      moveFocus(-1)
      return
    }
    if (key.name === "down" || key.name === "j") {
      moveFocus(1)
      return
    }
    if (key.name === "enter" || key.name === "return") {
      submit()
      return
    }
    if (focus() === hexFocus()) {
      return
    }
    if (key.name === "left" || key.name === "h") {
      adjust(-1)
      return
    }
    if (key.name === "right" || key.name === "l") {
      adjust(1)
    }
  })

  const status = (): string => issue() ?? " "
  const statusTone = () => (issue() === null ? palette.faint : palette.danger)
  const hints = createMemo<readonly Hint[]>(() => [
    { keys: "esc", label: "revert" },
    { keys: "tab", label: "mode" },
    { keys: "↑↓", label: "field" },
    { keys: "←→", label: "adjust" },
    { keys: "enter", label: props.busy ? "applying…" : "done" },
  ])

  return (
    <box flexDirection="column" flexGrow={1} gap={0} minHeight={0}>
      <box flexDirection="row" justifyContent="space-between" height={1}>
        <text fg={palette.text}>
          <strong>Set colour</strong>
        </text>
        <text fg={palette.faint}>esc revert</text>
      </box>

      <ModeTabs mode={mode()} />

      <ColorSliders
        mode={mode()}
        focus={focus()}
        hsb={hsb()}
        rgb={rgb()}
        width={meterWidth()}
        onFocus={(next) => {
          setFocus(next)
        }}
      />

      <box height={1} />

      <box flexDirection="row" gap={1} height={1}>
        <text fg={focus() === hexFocus() ? palette.accent : palette.background}>
          {focus() === hexFocus() ? "›" : " "}
        </text>
        <box width={11}>
          <text fg={focus() === hexFocus() ? palette.accent : palette.muted}>Hex</text>
        </box>
        <input
          width={12}
          value={hexDraft()}
          onInput={editHex}
          focused={ready() && focus() === hexFocus()}
          maxLength={7}
          placeholder="#FFFFFF"
          backgroundColor={palette.background}
          textColor={palette.text}
          focusedBackgroundColor={palette.background}
          cursorColor={palette.accent}
          placeholderColor={palette.faint}
        />
        <text fg={palette.faint}>six digits</text>
      </box>

      <Swatch hex={hex()} />

      <box height={1}>
        <text fg={statusTone()} wrapMode="word">
          {status()}
        </text>
      </box>

      <box flexGrow={1} />

      <KeyBar hints={hints()} />
    </box>
  )
}

export { Color }
