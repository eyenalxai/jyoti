import { useKeyboard } from "@opentui/solid"
import { Effect } from "effect"
import { createMemo, createSignal, onCleanup, onMount } from "solid-js"

import type { PickerFocus } from "@/components/bulb/color/focus"
import type { Mode } from "@/components/bulb/color/mode"
import type { Oklch } from "@/lib/color"
import type { Timer } from "@/lib/timers"

import { HEX_FOCUS, focusOrder } from "@/components/bulb/color/focus"
import { ModeTabs } from "@/components/bulb/color/mode"
import { ColorSliders } from "@/components/bulb/color/sliders"
import { Swatch } from "@/components/bulb/color/swatch"
import {
  CHROMA_RANGE,
  HUE_RANGE,
  LIGHTNESS_RANGE,
  hexToOklch,
  oklchToSrgb,
  rgbToOklch,
} from "@/lib/color"
import { clamp } from "@/lib/format"
import { palette } from "@/lib/theme"
import { after, cancel } from "@/lib/timers"

interface ColorProps {
  readonly initial: string
  readonly busy: boolean
  readonly onSubmit: (hex: string) => void
  readonly onPreview: (hex: string) => Promise<boolean>
  readonly onDismiss: () => void
}

const PREVIEW_DELAY_MS = 180
const WHITE: Oklch = { lightness: 100, chroma: 0, hue: 0 }

const Color = (props: ColorProps) => {
  const seeded = hexToOklch(props.initial) ?? WHITE
  const seededHex = oklchToSrgb(seeded).hex
  const [color, setColor] = createSignal<Oklch>(seeded)
  const [mode, setMode] = createSignal<Mode>("oklch")
  const [focus, setFocus] = createSignal<PickerFocus>(0)
  const [hexDraft, setHexDraft] = createSignal(seededHex)
  const [lastSent, setLastSent] = createSignal(seededHex)
  const [issue, setIssue] = createSignal<string | null>(null)
  const [ready, setReady] = createSignal(false)
  let previewTimer: Timer | null = null

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

  const preview = createMemo(() => oklchToSrgb(color()))
  const rgb = () => preview().rgb
  const hex = () => preview().hex
  const clipped = () => preview().clipped

  const sendPreview = Effect.fn("Color.sendPreview")(function* sendPreview(next: string) {
    const applied = yield* Effect.promise(() => props.onPreview(next))
    if (applied && hex() === next) {
      setLastSent(next)
    }
  })

  const schedulePreview = () => {
    if (previewTimer !== null) {
      cancel(previewTimer)
    }
    previewTimer = after(PREVIEW_DELAY_MS, () => {
      previewTimer = null
      const next = hex()
      if (next === lastSent()) {
        return
      }
      Effect.runFork(sendPreview(next).pipe(Effect.ignoreCause))
    })
  }

  const applyColor = (next: Oklch) => {
    setColor(next)
    setHexDraft(oklchToSrgb(next).hex)
    setIssue(null)
    schedulePreview()
  }

  const adjust = (direction: number) => {
    if (mode() === "oklch") {
      const current = color()
      if (focus() === 0) {
        applyColor({
          ...current,
          lightness: clamp(
            current.lightness + direction * LIGHTNESS_RANGE.step,
            LIGHTNESS_RANGE.min,
            LIGHTNESS_RANGE.max,
          ),
        })
      }
      if (focus() === 1) {
        applyColor({
          ...current,
          chroma: clamp(
            current.chroma + direction * CHROMA_RANGE.step,
            CHROMA_RANGE.min,
            CHROMA_RANGE.max,
          ),
        })
      }
      if (focus() === 2) {
        applyColor({
          ...current,
          hue: clamp(current.hue + direction * HUE_RANGE.step, HUE_RANGE.min, HUE_RANGE.max),
        })
      }
      return
    }

    const current = rgb()
    if (focus() === 0) {
      applyColor(rgbToOklch({ ...current, red: clamp(current.red + direction, 0, 255) }))
    }
    if (focus() === 1) {
      applyColor(rgbToOklch({ ...current, green: clamp(current.green + direction, 0, 255) }))
    }
    if (focus() === 2) {
      applyColor(rgbToOklch({ ...current, blue: clamp(current.blue + direction, 0, 255) }))
    }
  }

  const moveFocus = (direction: number) => {
    const current = focusOrder.indexOf(focus())
    const next = focusOrder[(current + direction + focusOrder.length) % focusOrder.length]
    if (next === undefined) {
      return
    }
    if (next === HEX_FOCUS) {
      setHexDraft(hex())
    }
    setFocus(next)
  }

  const switchMode = () => {
    setMode((current) => (current === "oklch" ? "rgb" : "oklch"))
    setHexDraft(hex())
    setIssue(null)
  }

  const editHex = (next: string) => {
    setHexDraft(next)
    // The input emits `input` when its value prop changes, so ignore the canonical echo.
    if (next === hex()) {
      return
    }
    const parsed = hexToOklch(next)
    if (parsed !== null) {
      setColor(parsed)
      setIssue(null)
      schedulePreview()
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
    const parsed = hexToOklch(hexDraft())
    if (parsed === null) {
      setIssue("Enter six hexadecimal digits, for example #FF8800.")
      return
    }
    setIssue(null)
    const next = oklchToSrgb(parsed).hex
    if (next === lastSent()) {
      props.onDismiss()
      return
    }
    setColor(parsed)
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
    if (focus() === HEX_FOCUS) {
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

  const status = (): string =>
    issue() ?? (clipped() ? "Chroma reduced until the colour fits sRGB." : " ")
  const statusTone = () => {
    if (issue() !== null) {
      return palette.danger
    }
    return clipped() ? palette.warning : palette.background
  }

  return (
    <box flexDirection="column" flexGrow={1} gap={0}>
      <ModeTabs mode={mode()} />

      <ColorSliders
        mode={mode()}
        focus={focus()}
        color={color()}
        rgb={rgb()}
        onFocus={(next) => {
          setFocus(next)
        }}
      />

      <box height={1} />

      <box flexDirection="row" gap={1} height={1}>
        <text fg={focus() === HEX_FOCUS ? palette.accent : palette.background}>
          {focus() === HEX_FOCUS ? "›" : " "}
        </text>
        <box width={6}>
          <text fg={focus() === HEX_FOCUS ? palette.accent : palette.muted}>Hex</text>
        </box>
        <input
          width={12}
          value={hexDraft()}
          onInput={editHex}
          focused={ready() && focus() === HEX_FOCUS}
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

      <Swatch hex={hex()} clipped={clipped()} />

      <box height={1}>
        <text fg={statusTone()} truncate>
          {status()}
        </text>
      </box>

      <box flexDirection="row" justifyContent="space-between" height={1}>
        <text bg={palette.surfaceRaised} fg={palette.faint}>
          esc revert · tab mode · ↑↓ move
        </text>
        <text bg={palette.surfaceRaised} fg={props.busy ? palette.faint : palette.accent}>
          {props.busy ? "applying…" : "enter done"}
        </text>
      </box>
    </box>
  )
}

export { Color }
