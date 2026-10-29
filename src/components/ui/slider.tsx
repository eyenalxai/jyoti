import { String as Str } from "effect"

import { Meter } from "@/components/ui/meter"
import { palette } from "@/lib/theme"

interface SliderProps {
  readonly label: string
  readonly value: number
  readonly min: number
  readonly max: number
  readonly text: string
  readonly width: number
  readonly focused: boolean
  readonly onFocus: () => void
}

const Slider = (props: SliderProps) => (
  <box flexDirection="row" gap={1} height={1} onMouseDown={props.onFocus}>
    <text fg={props.focused ? palette.accent : palette.background}>
      {props.focused ? "›" : " "}
    </text>
    <box width={6}>
      <text fg={props.focused ? palette.accent : palette.muted}>{props.label}</text>
    </box>
    <Meter
      value={props.value}
      min={props.min}
      max={props.max}
      width={props.width}
      color={props.focused ? palette.accent : palette.muted}
    />
    <text fg={props.focused ? palette.text : palette.muted}>
      {Str.padStart(6, " ")(props.text)}
    </text>
  </box>
)

export { Slider }
