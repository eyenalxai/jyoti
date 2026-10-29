import { Show } from "solid-js"

import type { Mode } from "@/components/bulb/color/mode"
import type { Rgb } from "@/lib/bulbs/schema"
import type { ChannelRange } from "@/lib/color"
import type { Hsb } from "@/lib/color/hsb"

import { Slider } from "@/components/ui/slider"
import { HUE_MAX, PERCENT_MAX } from "@/lib/color/hsb"

type PickerFocus = 0 | 1 | 2 | 3

const RGB_RANGE: ChannelRange = { min: 0, max: 255, step: 1 }
const HUE_RANGE: ChannelRange = { min: 0, max: HUE_MAX, step: 1 }
const SATURATION_RANGE: ChannelRange = { min: 0, max: PERCENT_MAX, step: 1 }

const formatHue = (value: number): string => `${Math.round(value)}°`
const formatPercent = (value: number): string => `${Math.round(value)}%`
const formatChannel = (value: number): string => `${value}`

interface ColorSlidersProps {
  readonly mode: Mode
  readonly focus: PickerFocus
  readonly hsb: Hsb
  readonly rgb: Rgb
  readonly width: number
  readonly onFocus: (focus: PickerFocus) => void
}

const ColorSliders = (props: ColorSlidersProps) => {
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
      width={props.width}
      labelWidth={11}
      focused={props.focus === index}
      onFocus={() => {
        props.onFocus(index)
      }}
    />
  )

  return (
    <box flexDirection="column" height={props.mode === "hs" ? 2 : 3}>
      <Show
        when={props.mode === "hs"}
        fallback={
          <>
            {slider(0, "Red", props.rgb.red, RGB_RANGE, formatChannel(props.rgb.red))}
            {slider(1, "Green", props.rgb.green, RGB_RANGE, formatChannel(props.rgb.green))}
            {slider(2, "Blue", props.rgb.blue, RGB_RANGE, formatChannel(props.rgb.blue))}
          </>
        }
      >
        {slider(0, "Hue", props.hsb.hue, HUE_RANGE, formatHue(props.hsb.hue))}
        {slider(
          1,
          "Saturation",
          props.hsb.saturation,
          SATURATION_RANGE,
          formatPercent(props.hsb.saturation),
        )}
      </Show>
    </box>
  )
}

export { ColorSliders, type PickerFocus }
