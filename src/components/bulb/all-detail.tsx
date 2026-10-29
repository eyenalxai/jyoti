import type { RGBA } from "@opentui/core"

import { String as Str } from "effect"
import { Match, Show, Switch } from "solid-js"

import type { ControlFocus } from "@/components/bulb/controls"
import type {
  AggregateColor,
  AggregatePower,
  AggregateState,
  ValueSpan,
} from "@/lib/bulbs/aggregate"

import { ControlRow } from "@/components/bulb/controls"
import { Meter, RangeMeter } from "@/components/ui/meter"
import {
  BRIGHTNESS_MAX,
  BRIGHTNESS_MIN,
  TEMPERATURE_MAX,
  TEMPERATURE_MIN,
} from "@/lib/bulbs/schema"
import {
  aggregateColorWord,
  aggregateGlyph,
  aggregateSummary,
  spanText,
  truncate,
} from "@/lib/format"
import { palette } from "@/lib/theme"

const aggregatePowerTone = (power: AggregatePower): RGBA => {
  if (power === "on") {
    return palette.accent
  }
  if (power === "mixed") {
    return palette.warning
  }
  if (power === "off") {
    return palette.muted
  }
  return palette.faint
}

const spanValue = (span: ValueSpan | null, suffix: string, width: number): string =>
  Str.padStart(width, " ")(spanText(span, suffix))

const SpanMeter = (props: {
  readonly span: ValueSpan | null
  readonly min: number
  readonly max: number
  readonly width: number
}) => (
  <Switch
    fallback={
      <RangeMeter
        low={props.span?.low ?? props.min}
        high={props.span?.high ?? props.max}
        min={props.min}
        max={props.max}
        width={props.width}
      />
    }
  >
    <Match when={props.span === null}>
      <Meter value={null} min={props.min} max={props.max} width={props.width} />
    </Match>
    <Match when={props.span !== null && props.span.low === props.span.high}>
      <Meter value={props.span?.low ?? null} min={props.min} max={props.max} width={props.width} />
    </Match>
  </Switch>
)

const ColorSwatch = (props: { readonly color: AggregateColor | null; readonly width: number }) => {
  const blocks = () => Str.repeat(Math.max(4, Math.min(12, props.width)))("█")
  const tone = (): string | RGBA => {
    const color = props.color
    return color?.kind === "equal" ? color.hex : palette.text
  }
  return (
    <Switch fallback={<text fg={palette.border}>{blocks()}</text>}>
      <Match when={props.color?.kind === "mixed"}>
        <text fg={palette.warning}>colors differ</text>
      </Match>
      <Match when={props.color?.kind === "equal"}>
        <text fg={tone()}>
          {blocks()}
          <span style={{ fg: palette.faint }}> same color</span>
        </text>
      </Match>
    </Switch>
  )
}

interface AllDetailProps {
  readonly aggregate: AggregateState
  readonly focus: ControlFocus
  readonly active: boolean
  readonly width: number
  readonly meterWidth: number
  readonly onFocus: (focus: ControlFocus) => void
}

const AllDetail = (props: AllDetailProps) => {
  const reachLine = (): string => {
    const aggregate = props.aggregate
    if (aggregate.failed > 0) {
      return `${aggregate.reachable} of ${aggregate.total} reachable`
    }
    if (aggregate.reachable < aggregate.total) {
      return "reading…"
    }
    return `${aggregate.total} ${aggregate.total === 1 ? "bulb" : "bulbs"} · all reachable`
  }

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
        {reachLine()}
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
          detail={<text fg={palette.faint}>space toggles all</text>}
        />
        <ControlRow
          label="Brightness"
          focused={props.active && props.focus === "brightness"}
          onFocus={() => {
            props.onFocus("brightness")
          }}
          value={<text fg={palette.text}>{spanValue(props.aggregate.brightness, "%", 7)}</text>}
          detail={
            <SpanMeter
              span={props.aggregate.brightness}
              min={BRIGHTNESS_MIN}
              max={BRIGHTNESS_MAX}
              width={props.meterWidth}
            />
          }
        />
        <ControlRow
          label="Warmth"
          focused={props.active && props.focus === "temperature"}
          onFocus={() => {
            props.onFocus("temperature")
          }}
          value={<text fg={palette.text}>{spanValue(props.aggregate.temperature, "K", 10)}</text>}
          detail={
            <SpanMeter
              span={props.aggregate.temperature}
              min={TEMPERATURE_MIN}
              max={TEMPERATURE_MAX}
              width={props.meterWidth}
            />
          }
        />
        <ControlRow
          label="Color"
          focused={props.active && props.focus === "color"}
          onFocus={() => {
            props.onFocus("color")
          }}
          value={
            <text fg={props.aggregate.color?.kind === "mixed" ? palette.warning : palette.text}>
              {aggregateColorWord(props.aggregate.color)}
            </text>
          }
          detail={<ColorSwatch color={props.aggregate.color} width={props.meterWidth} />}
        />
      </box>
    </box>
  )
}

export { AllDetail, type AllDetailProps }
