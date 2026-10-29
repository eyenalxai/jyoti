import type { RGBA } from "@opentui/core"

import { String as Str } from "effect"
import { Match, Switch } from "solid-js"

import type { AggregatePower, ValueSpan } from "@/lib/bulbs/aggregate"

import { Meter, RangeMeter } from "@/components/ui/meter"
import { spanText } from "@/lib/format"
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
  readonly disabled: boolean
}) => (
  <Switch
    fallback={
      <RangeMeter
        low={props.span?.low ?? props.min}
        high={props.span?.high ?? props.max}
        min={props.min}
        max={props.max}
        width={props.width}
        color={props.disabled ? palette.faint : palette.accent}
      />
    }
  >
    <Match when={props.span === null}>
      <Meter
        value={null}
        min={props.min}
        max={props.max}
        width={props.width}
        color={props.disabled ? palette.faint : palette.accent}
      />
    </Match>
    <Match when={props.span !== null && props.span.low === props.span.high}>
      <Meter
        value={props.span?.low ?? null}
        min={props.min}
        max={props.max}
        width={props.width}
        color={props.disabled ? palette.faint : palette.accent}
      />
    </Match>
  </Switch>
)

export { SpanMeter, aggregatePowerTone, spanValue }
