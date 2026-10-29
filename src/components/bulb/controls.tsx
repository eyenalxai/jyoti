import type { JSX } from "solid-js"

import { Show } from "solid-js"

import { palette } from "@/lib/theme"

type ControlFocus = "power" | "quality" | "brightness" | "light"

interface ControlRowProps {
  readonly label: string
  readonly value: JSX.Element
  readonly hint?: string | undefined
  readonly focused: boolean
  readonly disabled?: boolean
  readonly labelWidth: number
  readonly onFocus?: (() => void) | undefined
}

const ControlRow = (props: ControlRowProps) => {
  const disabled = () => props.disabled === true
  const active = () => props.focused && !disabled()

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
      <text fg={active() ? palette.accent : palette.background}>{props.focused ? "›" : " "}</text>
      <box width={props.labelWidth} flexShrink={0}>
        <text fg={active() ? palette.accent : palette.muted} truncate wrapMode="none">
          {props.label}
        </text>
      </box>
      <box flexDirection="row" gap={1} flexGrow={1} minWidth={0}>
        {props.value}
      </box>
      <Show when={props.hint}>
        {(hint) => (
          <text fg={palette.faint} flexShrink={0} wrapMode="none">
            {hint()}
          </text>
        )}
      </Show>
    </box>
  )
}

export { ControlRow, type ControlFocus, type ControlRowProps }
