import { useKeyboard } from "@opentui/solid"
import { Show, createMemo, createSignal, onCleanup, onMount } from "solid-js"

import type { Mode } from "@/components/bulb/color/mode"
import type { ChannelRange, Oklch } from "@/lib/color"

import { ModeTabs } from "@/components/bulb/color/mode"
import { Swatch } from "@/components/bulb/color/swatch"
import { Slider } from "@/components/ui/slider"
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
  readonly error: string | null
  readonly onSubmit: (hex: string) => void
}

type PickerFocus = 0 | 1 | 2 | 3

const HEX_FOCUS: PickerFocus = 3
const focusOrder: readonly PickerFocus[] = [0, 1, 2, HEX_FOCUS]
const RGB_RANGE: ChannelRange = { min: 0, max: 255, step: 1 }
const WHITE: Oklch = { lightness: 100, chroma: 0, hue: 0 }

const formatLightness = (value: number): string => `${Math.round(value)}`
const formatChroma = (value: number): string => value.toFixed(2)
const formatHue = (value: number): string => `${Math.round(value)}°`
const formatChannel = (value: number): string => `${value}`

const Color = (props: ColorProps) => {
  const seeded = hexToOklch(props.initial) ?? WHITE
  const [color, setColor] = createSignal<Oklch>(seeded)
  const [mode, setMode] = createSignal<Mode>("oklch")
  const [focus, setFocus] = createSignal<PickerFocus>(0)
  const [hexDraft, setHexDraft] = createSignal(oklchToSrgb(seeded).hex)
  const [issue, setIssue] = createSignal<string | null>(null)
  const [ready, setReady] = createSignal(false)

  onMount(() => {
    const timer = after(10, () => {
      setReady(true)
    })
    onCleanup(() => {
      cancel(timer)
    })
  })

  const preview = createMemo(() => oklchToSrgb(color()))
  const rgb = () => preview().rgb
  const hex = () => preview().hex
  const clipped = () => preview().clipped

  const applyColor = (next: Oklch) => {
    setColor(next)
    setHexDraft(oklchToSrgb(next).hex)
    setIssue(null)
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
    }
  }

  const submit = () => {
    if (props.busy) {
      return
    }
    const parsed = hexToOklch(hexDraft())
    if (parsed === null) {
      setIssue("Enter six hexadecimal digits, for example #FF8800.")
      return
    }
    setIssue(null)
    setColor(parsed)
    props.onSubmit(oklchToSrgb(parsed).hex)
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

  const slider = (
    index: PickerFocus,
    label: string,
    value: number,
    range: ChannelRange,
    text: string,
  ) => (
    <Slider
      label={label}
      value={value}
      min={range.min}
      max={range.max}
      text={text}
      width={27}
      focused={focus() === index}
      onFocus={() => {
        setFocus(index)
      }}
    />
  )

  const status = (): string =>
    props.error ?? issue() ?? (clipped() ? "Chroma reduced until the colour fits sRGB." : " ")
  const statusTone = () => {
    if (props.error !== null || issue() !== null) {
      return palette.danger
    }
    return clipped() ? palette.warning : palette.background
  }

  return (
    <box flexDirection="column" flexGrow={1} gap={0}>
      <ModeTabs mode={mode()} />

      <box flexDirection="column" height={3}>
        <Show
          when={mode() === "oklch"}
          fallback={
            <>
              {slider(0, "Red", rgb().red, RGB_RANGE, formatChannel(rgb().red))}
              {slider(1, "Green", rgb().green, RGB_RANGE, formatChannel(rgb().green))}
              {slider(2, "Blue", rgb().blue, RGB_RANGE, formatChannel(rgb().blue))}
            </>
          }
        >
          {slider(
            0,
            "Light",
            color().lightness,
            LIGHTNESS_RANGE,
            formatLightness(color().lightness),
          )}
          {slider(1, "Chroma", color().chroma, CHROMA_RANGE, formatChroma(color().chroma))}
          {slider(2, "Hue", color().hue, HUE_RANGE, formatHue(color().hue))}
        </Show>
      </box>

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
          esc cancel · tab mode · ↑↓ move
        </text>
        <text bg={palette.surfaceRaised} fg={props.busy ? palette.faint : palette.accent}>
          {props.busy ? "applying…" : "enter apply"}
        </text>
      </box>
    </box>
  )
}

export { Color }
