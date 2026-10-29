import type { ColorInput } from "@opentui/core"

import { String as Str } from "effect"
import { createMemo } from "solid-js"

import { meterUnits } from "@/lib/format"
import { palette } from "@/lib/theme"

interface MeterProps {
  readonly value: number | null
  readonly min: number
  readonly max: number
  readonly width: number
  readonly color?: ColorInput
}

const Meter = (props: MeterProps) => {
  const units = createMemo(() => meterUnits(props.value, props.min, props.max, props.width))
  const color = () => props.color ?? palette.accent

  return (
    <text height={1}>
      <span style={{ fg: color() }}>{Str.repeat(units())("█")}</span>
      <span style={{ fg: palette.border }}>
        {Str.repeat(Math.max(0, props.width - units()))("░")}
      </span>
    </text>
  )
}

interface RangeMeterProps {
  readonly low: number
  readonly high: number
  readonly min: number
  readonly max: number
  readonly width: number
  readonly color?: ColorInput
}

const RangeMeter = (props: RangeMeterProps) => {
  const bounds = createMemo(() => {
    const start = meterUnits(props.low, props.min, props.max, props.width)
    const end = Math.min(
      props.width,
      Math.max(start + 1, meterUnits(props.high, props.min, props.max, props.width)),
    )
    return { start, end }
  })
  const color = () => props.color ?? palette.accent

  return (
    <text height={1}>
      <span style={{ fg: palette.border }}>{Str.repeat(bounds().start)("░")}</span>
      <span style={{ fg: color() }}>{Str.repeat(bounds().end - bounds().start)("█")}</span>
      <span style={{ fg: palette.border }}>
        {Str.repeat(Math.max(0, props.width - bounds().end))("░")}
      </span>
    </text>
  )
}

export { Meter, RangeMeter }
