import type { Bulb, BulbState, ColorHex } from "@/lib/bulbs/schema"

interface ValueSpan {
  readonly low: number
  readonly high: number
}

type AggregatePower = "on" | "off" | "mixed" | "unknown"

type LightSide = "white" | "color"

type LightQuality = LightSide | "mixed" | "unknown"

type AggregateColor =
  | { readonly kind: "equal"; readonly hex: ColorHex }
  | { readonly kind: "mixed" }

interface AggregateState {
  readonly total: number
  readonly reachable: number
  readonly failed: number
  readonly power: AggregatePower
  readonly quality: LightQuality
  readonly brightness: ValueSpan | null
  readonly temperature: ValueSpan | null
  readonly color: AggregateColor | null
}

const spanOf = (
  values: readonly number[],
): { readonly low: number; readonly high: number } | null => {
  let span: ValueSpan | null = null
  for (const value of values) {
    span =
      span === null
        ? { low: value, high: value }
        : { low: Math.min(span.low, value), high: Math.max(span.high, value) }
  }
  return span
}

const absoluteStep = (span: ValueSpan, delta: number, min: number, max: number): number => {
  // Start at the aggregate edge the key moves toward, so no bulb moves against it.
  const anchor = delta > 0 ? span.high : span.low
  return Math.min(max, Math.max(min, anchor + delta))
}

const powerOf = (values: readonly boolean[]): AggregatePower => {
  if (values.length === 0) {
    return "unknown"
  }
  let on = 0
  for (const value of values) {
    if (value) {
      on += 1
    }
  }
  if (on === values.length) {
    return "on"
  }
  return on === 0 ? "off" : "mixed"
}

const colorOf = (values: readonly ColorHex[]): AggregateColor | null => {
  const first = values[0]
  if (first === undefined) {
    return null
  }
  for (const value of values) {
    if (value !== first) {
      return { kind: "mixed" }
    }
  }
  return { kind: "equal", hex: first }
}

const qualityOf = (values: readonly LightQuality[]): LightQuality => {
  const first = values[0]
  if (first === undefined) {
    return "unknown"
  }
  for (const value of values) {
    if (value !== first) {
      return "mixed"
    }
  }
  return first
}

const lightQualityOf = (mode: BulbState["mode"] | undefined): LightQuality => {
  if (mode === "temperature") {
    return "white"
  }
  if (mode === "color") {
    return "color"
  }
  return "unknown"
}

const aggregateState = (
  bulbs: readonly Bulb[],
  states: ReadonlyMap<string, BulbState>,
  stateErrors: ReadonlyMap<string, string>,
): AggregateState => {
  const powers: boolean[] = []
  const qualities: LightQuality[] = []
  const brightness: number[] = []
  const temperatures: number[] = []
  const colors: ColorHex[] = []
  let reachable = 0
  let failed = 0

  for (const bulb of bulbs) {
    const state = states.get(bulb.id)
    if (state === undefined) {
      if (stateErrors.has(bulb.id)) {
        failed += 1
      }
      continue
    }
    reachable += 1
    const quality = lightQualityOf(state.mode)
    if (quality !== "unknown") {
      qualities.push(quality)
    }
    if (state.power !== null) {
      powers.push(state.power)
    }
    if (state.brightness !== null) {
      brightness.push(state.brightness)
    }
    if (state.temperature !== null) {
      temperatures.push(state.temperature)
    }
    if (state.color !== null) {
      colors.push(state.color)
    }
  }

  return {
    total: bulbs.length,
    reachable,
    failed,
    power: powerOf(powers),
    quality: qualityOf(qualities),
    brightness: spanOf(brightness),
    temperature: spanOf(temperatures),
    color: colorOf(colors),
  }
}

export {
  absoluteStep,
  aggregateState,
  lightQualityOf,
  type AggregateColor,
  type AggregatePower,
  type LightQuality,
  type LightSide,
  type AggregateState,
  type ValueSpan,
}
