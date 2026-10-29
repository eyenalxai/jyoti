import { useKeyboard } from "@opentui/solid"
import { Show, createMemo, createSignal, onCleanup, onMount } from "solid-js"

import type { Mode } from "@/components/bulb/color/mode"
import type { Rgb } from "@/lib/bulbs/schema"
import type { ChannelRange } from "@/lib/color"

import { ModeTabs } from "@/components/bulb/color/mode"
import { Swatch } from "@/components/bulb/color/swatch"
import { Slider } from "@/components/ui/slider"
import { formatColor, packColor, parseColor } from "@/lib/bulbs/schema"
import { HUE_MAX, PERCENT_MAX, hsbToRgb, rgbToHsb, withHue, withSaturation } from "@/lib/color/hsb"
import { clamp } from "@/lib/format"
import { palette } from "@/lib/theme"
import { after, cancel } from "@/lib/timers"

interface ColorProps {
  readonly initial: string
  readonly busy: boolean
  readonly error: string | null
  readonly width: number
  readonly onPreview: (hex: string) => void
  readonly onSubmit: () => void
}

type PickerFocus = 0 | 1 | 2 | 3

const RGB_RANGE: ChannelRange = { min: 0, max: 255, step: 1 }
const HUE_RANGE: ChannelRange = { min: 0, max: HUE_MAX, step: 1 }
const SATURATION_RANGE: ChannelRange = { min: 0, max: PERCENT_MAX, step: 1 }
const WHITE: Rgb = { red: 255, green: 255, blue: 255 }

const formatHue = (value: number): string => `${Math.round(value)}°`
const formatPercent = (value: number): string => `${Math.round(value)}%`
const formatChannel = (value: number): string => `${value}`

const Color = (props: ColorProps) => {
  const seeded = parseColor(props.initial) ?? WHITE
  const [rgb, setRgb] = createSignal<Rgb>(seeded)
  const [mode, setMode] = createSignal<Mode>("hs")
  const [focus, setFocus] = createSignal<PickerFocus>(0)
  const [hexDraft, setHexDraft] = createSignal(formatColor(packColor(seeded)))
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

  const hsb = createMemo(() => rgbToHsb(rgb()))
  const hex = () => formatColor(packColor(rgb()))
  const hexFocus = (): PickerFocus => (mode() === "hs" ? 2 : 3)
  const order = (): readonly PickerFocus[] => (mode() === "hs" ? [0, 1, 2] : [0, 1, 2, 3])
  const meterWidth = (): number => Math.max(10, Math.min(28, props.width - 30))

  const applyRgb = (next: Rgb, preview = true) => {
    setRgb(next)
    setHexDraft(formatColor(packColor(next)))
    setIssue(null)
    if (preview) {
      props.onPreview(formatColor(packColor(next)))
    }
  }

  const adjust = (direction: number) => {
    if (mode() === "hs") {
      const current = hsb()
      if (focus() === 0) {
        const base =
          current.saturation === 0 ? hsbToRgb({ ...current, saturation: PERCENT_MAX }) : rgb()
        applyRgb(withHue(base, clamp(current.hue + direction, HUE_RANGE.min, HUE_RANGE.max)))
      }
      if (focus() === 1) {
        applyRgb(
          withSaturation(
            rgb(),
            clamp(current.saturation + direction, SATURATION_RANGE.min, SATURATION_RANGE.max),
          ),
        )
      }
      return
    }

    const current = rgb()
    if (focus() === 0) {
      applyRgb({ ...current, red: clamp(current.red + direction, RGB_RANGE.min, RGB_RANGE.max) })
    }
    if (focus() === 1) {
      applyRgb({
        ...current,
        green: clamp(current.green + direction, RGB_RANGE.min, RGB_RANGE.max),
      })
    }
    if (focus() === 2) {
      applyRgb({ ...current, blue: clamp(current.blue + direction, RGB_RANGE.min, RGB_RANGE.max) })
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
    const parsed = parseColor(hexDraft())
    if (parsed === null) {
      setIssue("Enter six hexadecimal digits, for example #FF8800.")
      return
    }
    setIssue(null)
    const next = formatColor(packColor(parsed))
    if (next !== hex()) {
      applyRgb(parsed)
    }
    props.onSubmit()
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
      width={meterWidth()}
      labelWidth={11}
      focused={focus() === index}
      onFocus={() => {
        setFocus(index)
      }}
    />
  )

  const status = (): string => props.error ?? issue() ?? " "
  const statusTone = () =>
    props.error !== null || issue() !== null ? palette.danger : palette.faint

  return (
    <box flexDirection="column" flexGrow={1} gap={0} minHeight={0}>
      <box flexDirection="row" justifyContent="space-between" height={1}>
        <text fg={palette.text}>
          <strong>Set colour</strong>
        </text>
        <text fg={palette.faint}>esc close</text>
      </box>

      <ModeTabs mode={mode()} />

      <box flexDirection="column" height={mode() === "hs" ? 2 : 3}>
        <Show
          when={mode() === "hs"}
          fallback={
            <>
              {slider(0, "Red", rgb().red, RGB_RANGE, formatChannel(rgb().red))}
              {slider(1, "Green", rgb().green, RGB_RANGE, formatChannel(rgb().green))}
              {slider(2, "Blue", rgb().blue, RGB_RANGE, formatChannel(rgb().blue))}
            </>
          }
        >
          {slider(0, "Hue", hsb().hue, HUE_RANGE, formatHue(hsb().hue))}
          {slider(
            1,
            "Saturation",
            hsb().saturation,
            SATURATION_RANGE,
            formatPercent(hsb().saturation),
          )}
        </Show>
      </box>

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
        <text fg={statusTone()} truncate>
          {status()}
        </text>
      </box>

      <box flexDirection="row" justifyContent="space-between" height={1}>
        <text bg={palette.surfaceRaised} fg={palette.faint}>
          tab mode · ↑↓ field · ←→ adjust
        </text>
        <text bg={palette.surfaceRaised} fg={props.busy ? palette.faint : palette.accent}>
          {props.busy ? "applying…" : "enter done"}
        </text>
      </box>
    </box>
  )
}

export { Color }
