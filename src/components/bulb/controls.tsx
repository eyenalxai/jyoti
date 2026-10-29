import type { JSX } from "solid-js"

import { Show } from "solid-js"

import { palette } from "@/lib/theme"

type ControlFocus = "power" | "brightness" | "light"

interface ControlRowProps {
  readonly label: string
  readonly value: JSX.Element
  readonly hint?: string
  readonly focused: boolean
  readonly disabled?: boolean
  readonly onFocus?: (() => void) | undefined
}

const ControlRow = (props: ControlRowProps) => {
  const disabled = () => props.disabled === true

  return (
    <box
      flexDirection="row"
      gap={1}
      height={1}
      onMouseDown={() => {
        if (!disabled()) {
          props.onFocus?.()
        }
      }}
    >
      <text fg={props.focused && !disabled() ? palette.accent : palette.background}>
        {props.focused ? "›" : " "}
      </text>
      <box width={12} flexShrink={0}>
        <text fg={props.focused && !disabled() ? palette.accent : palette.muted}>
          {props.label}
        </text>
      </box>
      <box flexDirection="row" gap={1} flexGrow={1} minWidth={0}>
        {props.value}
      </box>
      <Show when={props.hint}>
        <text fg={palette.faint} flexShrink={0}>
          {props.hint}
        </text>
      </Show>
    </box>
  )
}

export { ControlRow, type ControlFocus, type ControlRowProps }
