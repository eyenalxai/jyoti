import { Show } from "solid-js"

import type { LightQuality, LightSide } from "@/lib/bulbs/aggregate"

import { palette } from "@/lib/theme"

interface QualitySwitchProps {
  readonly quality: LightQuality
  readonly disabled: boolean
  readonly onQuality: (quality: LightSide) => void
}

const Segment = (props: {
  readonly label: string
  readonly active: boolean
  readonly disabled: boolean
  readonly onSelect: () => void
}) => (
  <box
    onMouseDown={() => {
      if (!props.disabled) {
        props.onSelect()
      }
    }}
  >
    <text
      bg={props.active ? palette.accent : palette.background}
      fg={props.disabled ? palette.faint : props.active ? palette.background : palette.muted}
    >
      {` ${props.label} `}
    </text>
  </box>
)

const QualitySwitch = (props: QualitySwitchProps) => (
  <box flexDirection="row" justifyContent="space-between" height={1} gap={2}>
    <text fg={palette.muted}>Light</text>
    <box flexDirection="row" gap={1} alignItems="center">
      <Segment
        label="White"
        active={props.quality === "white"}
        disabled={props.disabled}
        onSelect={() => {
          props.onQuality("white")
        }}
      />
      <text fg={palette.border}>│</text>
      <Segment
        label="Colour"
        active={props.quality === "color"}
        disabled={props.disabled}
        onSelect={() => {
          props.onQuality("color")
        }}
      />
      <Show when={props.quality === "mixed"}>
        <text fg={palette.warning}>mixed</text>
      </Show>
      <Show when={props.quality === "unknown"}>
        <text fg={palette.faint}>unknown</text>
      </Show>
    </box>
  </box>
)

export { QualitySwitch }
