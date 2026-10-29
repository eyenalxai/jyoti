import type { RGBA } from "@opentui/core"

import { String as Str } from "effect"
import { Show } from "solid-js"

import type { ControlFocus } from "@/components/bulb/controls"
import type { LightSide } from "@/lib/bulbs/aggregate"
import type { Bulb, BulbState } from "@/lib/bulbs/schema"

import { Color } from "@/components/bulb/color"
import { ControlRow } from "@/components/bulb/controls"
import { QualitySwitch } from "@/components/bulb/quality"
import { Meter } from "@/components/ui/meter"
import { lightQualityOf } from "@/lib/bulbs/aggregate"
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
  readonly colorOpen: boolean
  readonly colorBusy: boolean
  readonly colorInitial: string
  readonly onFocus: (focus: ControlFocus) => void
  readonly onQuality: (quality: LightSide) => void
  readonly onPreviewColor: (hex: string) => Promise<boolean>
  readonly onSubmitColor: (hex: string) => void
  readonly onDismissColor: () => void
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
    hint="space"
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
      <>
        <Meter
          value={props.state?.brightness ?? null}
          min={BRIGHTNESS_MIN}
          max={BRIGHTNESS_MAX}
          width={props.meterWidth}
          color={props.disabled ? palette.faint : palette.accent}
        />
        <text fg={props.disabled ? palette.faint : palette.text}>
          {valueText(props.state?.brightness, "%", 4)}
        </text>
      </>
    }
    hint="←/→"
  />
)

const WarmthControl = (props: {
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
      <>
        <Meter
          value={props.state?.temperature ?? null}
          min={TEMPERATURE_MIN}
          max={TEMPERATURE_MAX}
          width={props.meterWidth}
          color={props.disabled ? palette.faint : palette.accent}
        />
        <text fg={props.disabled ? palette.faint : palette.text}>
          {valueText(props.state?.temperature, "K", 5)}
        </text>
      </>
    }
    hint="←/→"
  />
)

const ColourControl = (props: {
  readonly state: BulbState | undefined
  readonly focused: boolean
  readonly disabled: boolean
  readonly onFocus: () => void
}) => (
  <ControlRow
    label="Colour"
    focused={props.focused}
    disabled={props.disabled}
    onFocus={props.onFocus}
    value={
      <>
        <text fg={props.disabled ? palette.faint : (props.state?.color ?? palette.border)}>
          {Str.repeat(10)("█")}
        </text>
        <text fg={props.disabled ? palette.faint : palette.text}>{props.state?.color ?? "—"}</text>
      </>
    }
    hint={props.disabled ? "sync owns it" : "enter picks"}
  />
)

const UnknownQuality = (props: { readonly focused: boolean; readonly onFocus: () => void }) => (
  <ControlRow
    label="Quality"
    focused={props.focused}
    onFocus={props.onFocus}
    value={<text fg={palette.faint}>unknown</text>}
    hint="r reads it"
  />
)

const Detail = (props: DetailProps) => {
  const quality = () => lightQualityOf(props.state?.mode)
  const qualityFocused = () => props.active && props.focus === "light"
  const locked = () => props.syncLocked

  return (
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

      <Show
        when={!props.colorOpen}
        fallback={
          <Color
            initial={props.colorInitial}
            busy={props.colorBusy}
            width={props.width}
            onPreview={props.onPreviewColor}
            onSubmit={props.onSubmitColor}
            onDismiss={props.onDismissColor}
          />
        }
      >
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
            disabled={locked()}
            meterWidth={props.meterWidth}
            onFocus={() => {
              props.onFocus("brightness")
            }}
          />

          <text fg={palette.border} height={1}>
            {Str.repeat(Math.max(0, props.width))("─")}
          </text>

          <QualitySwitch quality={quality()} disabled={locked()} onQuality={props.onQuality} />

          <Show when={quality() === "white"}>
            <WarmthControl
              state={props.state}
              focused={qualityFocused()}
              disabled={locked()}
              meterWidth={props.meterWidth}
              onFocus={() => {
                props.onFocus("light")
              }}
            />
          </Show>
          <Show when={quality() === "color"}>
            <ColourControl
              state={props.state}
              focused={qualityFocused()}
              disabled={locked()}
              onFocus={() => {
                props.onFocus("light")
              }}
            />
          </Show>
          <Show when={quality() === "unknown"}>
            <UnknownQuality
              focused={qualityFocused()}
              onFocus={() => {
                props.onFocus("light")
              }}
            />
          </Show>
        </box>
      </Show>
    </box>
  )
}

export { Detail }
