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

export { Meter }
