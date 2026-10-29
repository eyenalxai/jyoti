import { Show } from "solid-js"

import type { LightQuality, LightSide } from "@/lib/bulbs/aggregate"

import { palette } from "@/lib/theme"

interface QualityTabsProps {
  readonly quality: LightQuality
  readonly focused: boolean
  readonly disabled: boolean
  readonly hint: string | undefined
  readonly onFocus: () => void
  readonly onSelect: (quality: LightSide) => void
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

const QualityTabs = (props: QualityTabsProps) => (
  <box flexDirection="row" gap={1} height={1} width="100%" onMouseDown={props.onFocus}>
    <text fg={props.focused && !props.disabled ? palette.accent : palette.background}>
      {props.focused ? "›" : " "}
    </text>
    <box flexDirection="row" gap={1}>
      <Segment
        label="White"
        active={props.quality === "white"}
        disabled={props.disabled}
        onSelect={() => {
          props.onSelect("white")
        }}
      />
      <Segment
        label="Colour"
        active={props.quality === "color"}
        disabled={props.disabled}
        onSelect={() => {
          props.onSelect("color")
        }}
      />
    </box>
    <Show when={props.quality === "mixed"}>
      <text fg={palette.warning}>mixed</text>
    </Show>
    <Show when={props.quality === "unknown"}>
      <text fg={palette.faint}>unknown</text>
    </Show>
    <box flexGrow={1} />
    <Show when={props.hint}>
      {(hint) => (
        <text fg={palette.faint} flexShrink={0} wrapMode="none">
          {hint()}
        </text>
      )}
    </Show>
  </box>
)

export { QualityTabs, type QualityTabsProps }
