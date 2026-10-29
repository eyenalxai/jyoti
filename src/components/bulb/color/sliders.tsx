import { Show } from "solid-js"

import type { PickerFocus } from "@/components/bulb/color/focus"
import type { Mode } from "@/components/bulb/color/mode"
import type { ChannelRange, Oklch, SrgbColor } from "@/lib/color"

import { Slider } from "@/components/ui/slider"
import { CHROMA_RANGE, HUE_RANGE, LIGHTNESS_RANGE } from "@/lib/color"

interface ColorSlidersProps {
  readonly mode: Mode
  readonly focus: PickerFocus
  readonly color: Oklch
  readonly rgb: SrgbColor["rgb"]
  readonly onFocus: (focus: PickerFocus) => void
}

const RGB_RANGE: ChannelRange = { min: 0, max: 255, step: 1 }

const formatLightness = (value: number): string => `${Math.round(value)}`
const formatChroma = (value: number): string => value.toFixed(2)
const formatHue = (value: number): string => `${Math.round(value)}°`
const formatChannel = (value: number): string => `${value}`

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
      width={27}
      focused={props.focus === index}
      onFocus={() => {
        props.onFocus(index)
      }}
    />
  )

  return (
    <box flexDirection="column" height={3}>
      <Show
        when={props.mode === "oklch"}
        fallback={
          <>
            {slider(0, "Red", props.rgb.red, RGB_RANGE, formatChannel(props.rgb.red))}
            {slider(1, "Green", props.rgb.green, RGB_RANGE, formatChannel(props.rgb.green))}
            {slider(2, "Blue", props.rgb.blue, RGB_RANGE, formatChannel(props.rgb.blue))}
          </>
        }
      >
        {slider(
          0,
          "Light",
          props.color.lightness,
          LIGHTNESS_RANGE,
          formatLightness(props.color.lightness),
        )}
        {slider(1, "Chroma", props.color.chroma, CHROMA_RANGE, formatChroma(props.color.chroma))}
        {slider(2, "Hue", props.color.hue, HUE_RANGE, formatHue(props.color.hue))}
      </Show>
    </box>
  )
}

export { ColorSliders }
