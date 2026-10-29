import type { RGBA } from "@opentui/core"

import { String as Str } from "effect"
import { Show } from "solid-js"

import type { ControlFocus } from "@/components/bulb/controls"
import type { Bulb, BulbState } from "@/lib/bulbs/schema"

import { ControlRow } from "@/components/bulb/controls"
import { Meter } from "@/components/ui/meter"
import {
  BRIGHTNESS_MAX,
  BRIGHTNESS_MIN,
  TEMPERATURE_MAX,
  TEMPERATURE_MIN,
} from "@/lib/bulbs/schema"
import { powerGlyph, powerWord, stateSummary, truncate } from "@/lib/format"
import { SYNC_OWNS_CONTROLS } from "@/lib/screen/sync/copy"
import { palette } from "@/lib/theme"

interface DetailProps {
  readonly bulb: Bulb
  readonly state: BulbState | undefined
  readonly error: string | undefined
  readonly focus: ControlFocus
  readonly active: boolean
  readonly syncLocked: boolean
  readonly width: number
  readonly meterWidth: number
  readonly onFocus: (focus: ControlFocus) => void
}

const valueText = (value: number | null | undefined, suffix: string, width: number): string =>
  value === null || value === undefined
    ? Str.padStart(width, " ")("—")
    : Str.padStart(width, " ")(`${value}${suffix}`)

const powerTone = (state: BulbState | undefined): RGBA =>
  state?.power === true ? palette.accent : palette.muted

const PowerControl = (props: {
  readonly state: BulbState | undefined
  readonly focused: boolean
  readonly onFocus: () => void
}) => (
  <ControlRow
    label="Power"
    focused={props.focused}
    onFocus={props.onFocus}
    value={<text fg={powerTone(props.state)}>{powerWord(props.state)}</text>}
    detail={<text fg={palette.faint}>space toggles</text>}
  />
)

const BrightnessControl = (props: {
  readonly state: BulbState | undefined
  readonly focused: boolean
  readonly disabled: boolean
  readonly meterWidth: number
  readonly onFocus: () => void
}) => (
  <ControlRow
    label="Brightness"
    focused={props.focused}
    disabled={props.disabled}
    onFocus={props.onFocus}
    value={
      <text fg={props.disabled ? palette.faint : palette.text}>
        {valueText(props.state?.brightness, "%", 4)}
      </text>
    }
    detail={
      <Meter
        value={props.state?.brightness ?? null}
        min={BRIGHTNESS_MIN}
        max={BRIGHTNESS_MAX}
        width={props.meterWidth}
        color={props.disabled ? palette.faint : palette.accent}
      />
    }
  />
)

const TemperatureControl = (props: {
  readonly state: BulbState | undefined
  readonly focused: boolean
  readonly disabled: boolean
  readonly meterWidth: number
  readonly onFocus: () => void
}) => (
  <ControlRow
    label="Warmth"
    focused={props.focused}
    disabled={props.disabled}
    onFocus={props.onFocus}
    value={
      <text fg={props.disabled ? palette.faint : palette.text}>
        {valueText(props.state?.temperature, "K", 5)}
      </text>
    }
    detail={
      <Meter
        value={props.state?.temperature ?? null}
        min={TEMPERATURE_MIN}
        max={TEMPERATURE_MAX}
        width={props.meterWidth}
        color={props.disabled ? palette.faint : palette.accent}
      />
    }
  />
)

const ColorControl = (props: {
  readonly state: BulbState | undefined
  readonly focused: boolean
  readonly disabled: boolean
  readonly meterWidth: number
  readonly onFocus: () => void
}) => (
  <ControlRow
    label="Color"
    focused={props.focused}
    disabled={props.disabled}
    onFocus={props.onFocus}
    value={
      <text fg={props.disabled ? palette.faint : palette.text}>{props.state?.color ?? "—"}</text>
    }
    detail={
      <text fg={props.disabled ? palette.faint : (props.state?.color ?? palette.border)}>
        {Str.repeat(Math.max(4, Math.min(12, props.meterWidth)))("█")}
        <span style={{ fg: palette.faint }}>
          {props.disabled ? " sync owns it" : " enter to set"}
        </span>
      </text>
    }
  />
)

const Detail = (props: DetailProps) => (
  <box flexDirection="column" flexGrow={1} minWidth={0} gap={0}>
    <box flexDirection="row" justifyContent="space-between" height={1}>
      <text fg={palette.text} truncate wrapMode="none">
        <strong>{truncate(props.bulb.name, Math.max(8, props.width - 12))}</strong>
      </text>
      <text fg={powerTone(props.state)}>
        {powerGlyph(props.state)} {powerWord(props.state)}
      </text>
    </box>

    <text fg={palette.muted} truncate wrapMode="none" height={1}>
      {props.bulb.address}
      {props.bulb.model === undefined ? "" : ` · ${props.bulb.model}`}
    </text>

    <box height={1}>
      <Show
        when={props.error === undefined}
        fallback={
          <text fg={palette.danger} truncate wrapMode="none">
            ✗ {props.error}
          </text>
        }
      >
        <text fg={palette.faint} truncate wrapMode="none">
          {stateSummary(props.state)}
        </text>
      </Show>
    </box>

    <text fg={palette.border} height={1}>
      {Str.repeat(Math.max(0, props.width))("─")}
    </text>

    <Show when={props.syncLocked}>
      <text fg={palette.warning} truncate wrapMode="none" height={1}>
        {SYNC_OWNS_CONTROLS}
      </text>
    </Show>

    <box flexDirection="column" flexGrow={1} minHeight={0}>
      <PowerControl
        state={props.state}
        focused={props.active && props.focus === "power"}
        onFocus={() => {
          props.onFocus("power")
        }}
      />
      <BrightnessControl
        state={props.state}
        focused={props.active && props.focus === "brightness"}
        disabled={props.syncLocked}
        meterWidth={props.meterWidth}
        onFocus={() => {
          props.onFocus("brightness")
        }}
      />
      <TemperatureControl
        state={props.state}
        focused={props.active && props.focus === "temperature"}
        disabled={props.syncLocked}
        meterWidth={props.meterWidth}
        onFocus={() => {
          props.onFocus("temperature")
        }}
      />
      <ColorControl
        state={props.state}
        focused={props.active && props.focus === "color"}
        disabled={props.syncLocked}
        meterWidth={props.meterWidth}
        onFocus={() => {
          props.onFocus("color")
        }}
      />
    </box>
  </box>
)

export { Detail }
