import { String as Str } from "effect"
import { Show } from "solid-js"

import type { ControlFocus } from "@/components/bulb/controls"
import type { AggregateColor, AggregateState, LightQuality, LightSide } from "@/lib/bulbs/aggregate"

import { SpanMeter, aggregatePowerTone, spanValue } from "@/components/bulb/aggregate-controls"
import { Color } from "@/components/bulb/color"
import { ControlRow } from "@/components/bulb/controls"
import { QualitySwitch } from "@/components/bulb/quality"
import {
  BRIGHTNESS_MAX,
  BRIGHTNESS_MIN,
  TEMPERATURE_MAX,
  TEMPERATURE_MIN,
} from "@/lib/bulbs/schema"
import { aggregateGlyph, aggregateSummary, truncate } from "@/lib/format"
import { SYNC_OWNS_CONTROLS } from "@/lib/screen/sync/copy"
import { palette } from "@/lib/theme"

const colorValue = (color: AggregateColor | null, disabled: boolean) => {
  if (color === null) {
    return <text fg={palette.faint}>—</text>
  }
  if (color.kind === "mixed") {
    return <text fg={palette.warning}>colours differ</text>
  }
  return (
    <>
      <text fg={disabled ? palette.faint : color.hex}>{Str.repeat(10)("█")}</text>
      <text fg={disabled ? palette.faint : palette.text}>{color.hex}</text>
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
  readonly colorOpen: boolean
  readonly colorBusy: boolean
  readonly colorInitial: string
  readonly onFocus: (focus: ControlFocus) => void
  readonly onQuality: (quality: LightSide) => void
  readonly onPreviewColor: (hex: string) => Promise<boolean>
  readonly onSubmitColor: (hex: string) => void
  readonly onDismissColor: () => void
}

const reachLine = (aggregate: AggregateState): string => {
  if (aggregate.failed > 0) {
    return `${aggregate.reachable} of ${aggregate.total} reachable`
  }
  if (aggregate.reachable < aggregate.total) {
    return "reading…"
  }
  return `${aggregate.total} ${aggregate.total === 1 ? "bulb" : "bulbs"} · all reachable`
}

const AllDetail = (props: AllDetailProps) => {
  const qualityFocused = () => props.active && props.focus === "light"
  const quality = (): LightQuality => props.aggregate.quality

  return (
    <box flexDirection="column" flexGrow={1} minWidth={0} gap={0}>
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
          <ControlRow
            label="Power"
            focused={props.active && props.focus === "power"}
            onFocus={() => {
              props.onFocus("power")
            }}
            value={
              <text fg={aggregatePowerTone(props.aggregate.power)}>{props.aggregate.power}</text>
            }
            hint="space"
          />
          <ControlRow
            label="Brightness"
            focused={props.active && props.focus === "brightness"}
            disabled={props.syncLocked}
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
                  disabled={props.syncLocked}
                />
                <text fg={props.syncLocked ? palette.faint : palette.text}>
                  {spanValue(props.aggregate.brightness, "%", 7)}
                </text>
              </>
            }
            hint="←/→"
          />

          <text fg={palette.border} height={1}>
            {Str.repeat(Math.max(0, props.width))("─")}
          </text>

          <QualitySwitch
            quality={quality()}
            disabled={props.syncLocked}
            onQuality={props.onQuality}
          />

          <Show when={quality() === "white"}>
            <ControlRow
              label="Warmth"
              focused={qualityFocused()}
              disabled={props.syncLocked}
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
                    disabled={props.syncLocked}
                  />
                  <text fg={props.syncLocked ? palette.faint : palette.text}>
                    {spanValue(props.aggregate.temperature, "K", 10)}
                  </text>
                </>
              }
              hint="←/→"
            />
          </Show>
          <Show when={quality() === "color"}>
            <ControlRow
              label="Colour"
              focused={qualityFocused()}
              disabled={props.syncLocked}
              onFocus={() => {
                props.onFocus("light")
              }}
              value={colorValue(props.aggregate.color, props.syncLocked)}
              hint={props.syncLocked ? "sync owns it" : "enter picks"}
            />
          </Show>
          <Show when={quality() === "mixed"}>
            <ControlRow
              label="Quality"
              focused={qualityFocused()}
              disabled={props.syncLocked}
              onFocus={() => {
                props.onFocus("light")
              }}
              value={<text fg={palette.warning}>mixed</text>}
              hint="w / c"
            />
          </Show>
          <Show when={quality() === "unknown"}>
            <ControlRow
              label="Quality"
              focused={qualityFocused()}
              disabled={props.syncLocked}
              onFocus={() => {
                props.onFocus("light")
              }}
              value={<text fg={palette.faint}>unknown</text>}
              hint="r reads"
            />
          </Show>
        </box>
      </Show>
    </box>
  )
}

export { AllDetail, type AllDetailProps }
