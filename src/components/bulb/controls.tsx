import type { JSX } from "solid-js"

import { palette } from "@/lib/theme"

type ControlFocus = "power" | "brightness" | "temperature" | "color"

interface ControlRowProps {
  readonly label: string
  readonly value: JSX.Element
  readonly detail: JSX.Element
  readonly focused: boolean
  readonly onFocus: () => void
}

const ControlRow = (props: ControlRowProps) => (
  <box flexDirection="column" height={2} onMouseDown={props.onFocus}>
    <box flexDirection="row" gap={1} height={1}>
      <text fg={props.focused ? palette.accent : palette.background}>
        {props.focused ? "›" : " "}
      </text>
      <box width={12}>
        <text fg={props.focused ? palette.accent : palette.muted}>{props.label}</text>
      </box>
      {props.value}
    </box>
    <box flexDirection="row" paddingLeft={2} height={1}>
      {props.detail}
    </box>
  </box>
)

export { ControlRow, type ControlFocus, type ControlRowProps }
