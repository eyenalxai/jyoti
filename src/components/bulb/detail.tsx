import type { RGBA } from "@opentui/core"

import { String as Str } from "effect"
import { Show } from "solid-js"

import type { ControlFocus } from "@/components/bulb/controls"
import type { LightSide } from "@/lib/bulbs/aggregate"
import type { Bulb, BulbState } from "@/lib/bulbs/schema"

import { Color } from "@/components/bulb/color"
import { ControlRow } from "@/components/bulb/controls"
import { QualityTabs } from "@/components/bulb/quality"
import { Meter } from "@/components/ui/meter"
import { Rule } from "@/components/ui/rule"
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
  readonly labelWidth: number
  readonly valueWidth: number
  readonly controlWidth: number
  readonly rowHints: boolean
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

const hintFor = (show: boolean, hint: string): string | undefined => (show ? hint : undefined)

const Detail = (props: DetailProps) => {
  const quality = () => lightQualityOf(props.state?.mode)
  const qualityFocused = () => props.active && props.focus === "quality"
  const valueFocused = () => props.active && props.focus === "light"
  const locked = () => props.syncLocked

  return (
    <box flexDirection="column" flexGrow={1} minWidth={0}>
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

      <Rule width={props.width} />

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

        <box flexDirection="column" flexGrow={1} minHeight={0} maxWidth={props.controlWidth}>
          <ControlRow
            label="Power"
            labelWidth={props.labelWidth}
            focused={props.active && props.focus === "power"}
            hint={hintFor(props.rowHints, "space")}
            onFocus={() => {
              props.onFocus("power")
            }}
            value={<text fg={powerTone(props.state)}>{powerWord(props.state)}</text>}
          />

          <box height={1} />

          <QualityTabs
            quality={quality()}
            focused={qualityFocused()}
            disabled={locked()}
            hint={hintFor(props.rowHints, "←/→")}
            onFocus={() => {
              props.onFocus("quality")
            }}
            onSelect={props.onQuality}
          />

          <ControlRow
            label="Brightness"
            labelWidth={props.labelWidth}
            focused={props.active && props.focus === "brightness"}
            disabled={locked()}
            hint={hintFor(props.rowHints, "←/→")}
            onFocus={() => {
              props.onFocus("brightness")
            }}
            value={
              <>
                <Meter
                  value={props.state?.brightness ?? null}
                  min={BRIGHTNESS_MIN}
                  max={BRIGHTNESS_MAX}
                  width={props.meterWidth}
                  color={locked() ? palette.faint : palette.accent}
                />
                <text fg={locked() ? palette.faint : palette.text} wrapMode="none">
                  {valueText(props.state?.brightness, "%", props.valueWidth)}
                </text>
              </>
            }
          />

          <Show when={quality() === "white"}>
            <ControlRow
              label="Warmth"
              labelWidth={props.labelWidth}
              focused={valueFocused()}
              disabled={locked()}
              hint={hintFor(props.rowHints, "←/→")}
              onFocus={() => {
                props.onFocus("light")
              }}
              value={
                <>
                  <Meter
                    value={props.state?.temperature ?? null}
                    min={TEMPERATURE_MIN}
                    max={TEMPERATURE_MAX}
                    width={props.meterWidth}
                    color={locked() ? palette.faint : palette.accent}
                  />
                  <text fg={locked() ? palette.faint : palette.text} wrapMode="none">
                    {valueText(props.state?.temperature, "K", props.valueWidth)}
                  </text>
                </>
              }
            />
          </Show>

          <Show when={quality() === "color"}>
            <ControlRow
              label="Colour"
              labelWidth={props.labelWidth}
              focused={valueFocused()}
              disabled={locked()}
              hint={hintFor(props.rowHints, "enter")}
              onFocus={() => {
                props.onFocus("light")
              }}
              value={
                <>
                  <text
                    fg={locked() ? palette.faint : (props.state?.color ?? palette.border)}
                    wrapMode="none"
                  >
                    {Str.repeat(Math.max(1, props.meterWidth))("█")}
                  </text>
                  <text fg={locked() ? palette.faint : palette.text} wrapMode="none">
                    {Str.padStart(props.valueWidth, " ")(props.state?.color ?? "—")}
                  </text>
                </>
              }
            />
          </Show>
        </box>
      </Show>
    </box>
  )
}

export { Detail, type DetailProps }
