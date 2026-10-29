import { String as Str } from "effect"
import { Show } from "solid-js"

import type { ControlFocus } from "@/components/bulb/controls"
import type { AggregateColor, AggregateState, LightSide } from "@/lib/bulbs/aggregate"

import { SpanMeter, aggregatePowerTone, spanValue } from "@/components/bulb/aggregate-controls"
import { Color } from "@/components/bulb/color"
import { ControlRow } from "@/components/bulb/controls"
import { QualityTabs } from "@/components/bulb/quality"
import { Rule } from "@/components/ui/rule"
import {
  BRIGHTNESS_MAX,
  BRIGHTNESS_MIN,
  TEMPERATURE_MAX,
  TEMPERATURE_MIN,
} from "@/lib/bulbs/schema"
import { aggregateGlyph, aggregateSummary, truncate } from "@/lib/format"
import { SYNC_OWNS_CONTROLS } from "@/lib/screen/sync/copy"
import { palette } from "@/lib/theme"

const reachLine = (aggregate: AggregateState): string => {
  if (aggregate.failed > 0) {
    return `${aggregate.reachable} of ${aggregate.total} reachable`
  }
  if (aggregate.reachable < aggregate.total) {
    return "reading…"
  }
  return `${aggregate.total} ${aggregate.total === 1 ? "bulb" : "bulbs"} · all reachable`
}

const colourValue = (color: AggregateColor | null, width: number, disabled: boolean) => {
  if (color === null) {
    return <text fg={palette.faint}>—</text>
  }
  if (color.kind === "mixed") {
    return <text fg={palette.warning}>colours differ</text>
  }
  return (
    <>
      <text fg={disabled ? palette.faint : color.hex} wrapMode="none">
        {Str.repeat(Math.max(1, width))("█")}
      </text>
    </>
  )
}

interface AllDetailProps {
  readonly aggregate: AggregateState
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

const hintFor = (show: boolean, hint: string): string | undefined => (show ? hint : undefined)

const AllDetail = (props: AllDetailProps) => {
  const qualityFocused = () => props.active && props.focus === "quality"
  const valueFocused = () => props.active && props.focus === "light"
  const locked = () => props.syncLocked

  return (
    <box flexDirection="column" flexGrow={1} minWidth={0}>
      <box flexDirection="row" justifyContent="space-between" height={1}>
        <text fg={palette.text} truncate wrapMode="none">
          <strong>{truncate("All bulbs", Math.max(8, props.width - 12))}</strong>
        </text>
        <text fg={aggregatePowerTone(props.aggregate.power)}>
          {aggregateGlyph(props.aggregate.power)} {props.aggregate.power}
        </text>
      </box>

      <text fg={palette.muted} truncate wrapMode="none" height={1}>
        {reachLine(props.aggregate)}
      </text>

      <box height={1}>
        <Show
          when={props.aggregate.failed > 0}
          fallback={
            <text fg={palette.faint} truncate wrapMode="none">
              {aggregateSummary(props.aggregate)}
            </text>
          }
        >
          <text
            fg={props.aggregate.reachable === 0 ? palette.danger : palette.warning}
            truncate
            wrapMode="none"
          >
            ✗ {props.aggregate.failed} of {props.aggregate.total} unreachable · r retry
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
            value={
              <text fg={aggregatePowerTone(props.aggregate.power)}>{props.aggregate.power}</text>
            }
          />

          <box height={1} />

          <QualityTabs
            quality={props.aggregate.quality}
            focused={qualityFocused()}
            disabled={locked()}
            hint={hintFor(props.rowHints, "←/→")}
            onFocus={() => {
              props.onFocus("quality")
            }}
            onSelect={props.onQuality}
          />

          <Show when={props.aggregate.quality === "white"}>
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
                  <SpanMeter
                    span={props.aggregate.brightness}
                    min={BRIGHTNESS_MIN}
                    max={BRIGHTNESS_MAX}
                    width={props.meterWidth}
                    disabled={locked()}
                  />
                  <text fg={locked() ? palette.faint : palette.text} wrapMode="none">
                    {spanValue(props.aggregate.brightness, "%", props.valueWidth)}
                  </text>
                </>
              }
            />
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
                  <SpanMeter
                    span={props.aggregate.temperature}
                    min={TEMPERATURE_MIN}
                    max={TEMPERATURE_MAX}
                    width={props.meterWidth}
                    disabled={locked()}
                  />
                  <text fg={locked() ? palette.faint : palette.text} wrapMode="none">
                    {spanValue(props.aggregate.temperature, "K", props.valueWidth)}
                  </text>
                </>
              }
            />
          </Show>

          <Show when={props.aggregate.quality === "color"}>
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
                  {colourValue(props.aggregate.color, props.meterWidth, locked())}
                  <text fg={locked() ? palette.faint : palette.text} wrapMode="none">
                    {Str.padStart(
                      props.valueWidth,
                      " ",
                    )(props.aggregate.color?.kind === "equal" ? props.aggregate.color.hex : "")}
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

export { AllDetail, type AllDetailProps }
