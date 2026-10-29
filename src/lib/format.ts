import { Array as Arr, String as Str } from "effect"

import type {
  AggregateColor,
  AggregatePower,
  AggregateState,
  ValueSpan,
} from "@/lib/bulbs/aggregate"
import type { BulbState } from "@/lib/bulbs/schema"

const truncate = (value: string, width: number): string => {
  if (width <= 0) {
    return ""
  }
  if (value.length <= width) {
    return value
  }
  if (width === 1) {
    return "…"
  }
  return `${Str.slice(0, width - 1)(value)}…`
}

const clamp = (value: number, min: number, max: number): number =>
  Math.min(max, Math.max(min, value))

const percentile = (values: Float32Array, fraction: number): number => {
  const sorted = Float32Array.from(values)
  sorted.sort()
  const index = Math.min(sorted.length - 1, Math.max(0, Math.floor((sorted.length - 1) * fraction)))
  return sorted[index] ?? 0
}

const meterUnits = (value: number | null, min: number, max: number, width: number): number => {
  if (value === null) {
    return 0
  }
  if (max <= min) {
    return 0
  }
  return clamp(Math.round(((value - min) / (max - min)) * width), 0, width)
}

const powerWord = (state: BulbState | undefined): string => {
  if (state === undefined) {
    return "unknown"
  }
  if (state.power === true) {
    return "on"
  }
  if (state.power === false) {
    return "off"
  }
  return "unknown"
}

const powerGlyph = (state: BulbState | undefined): string => {
  if (state === undefined) {
    return "◌"
  }
  if (state.power === true) {
    return "●"
  }
  if (state.power === false) {
    return "○"
  }
  return "◌"
}

const shortState = (state: BulbState | undefined): string => {
  if (state === undefined) {
    return "reading…"
  }
  if (state.power === false) {
    return "off"
  }
  if (state.power === null) {
    return "state unknown"
  }
  const brightness = state.brightness === null ? "?" : `${state.brightness}%`
  return `${powerWord(state)} · ${brightness}`
}

const stateSummary = (state: BulbState | undefined): string => {
  if (state === undefined) {
    return "Reading the bulb…"
  }
  if (state.power === null) {
    return "Power state unknown"
  }
  if (!state.power) {
    return "Off"
  }
  const parts = ["On"]
  if (state.brightness !== null) {
    parts.push(`${state.brightness}%`)
  }
  if (state.mode === "color" && state.color !== null) {
    parts.push(state.color)
  } else if (state.mode === "temperature" && state.temperature !== null) {
    parts.push(`${state.temperature}K`)
  }
  return Arr.join(parts, " · ")
}

const aggregateGlyph = (power: AggregatePower): string => {
  if (power === "on") {
    return "●"
  }
  if (power === "off") {
    return "○"
  }
  if (power === "mixed") {
    return "◐"
  }
  return "◌"
}

const spanText = (span: ValueSpan | null | undefined, suffix: string): string => {
  if (span === null || span === undefined) {
    return "—"
  }
  return span.low === span.high ? `${span.low}${suffix}` : `${span.low}–${span.high}${suffix}`
}

const aggregateColorWord = (color: AggregateColor | null): string => {
  if (color === null) {
    return "—"
  }
  return color.kind === "equal" ? color.hex : "mixed"
}

const nameList = (names: readonly string[]): string => {
  const first = names[0]
  const second = names[1]
  if (first === undefined) {
    return "The bulbs"
  }
  if (second === undefined) {
    return first
  }
  return names.length === 2
    ? `${first} and ${second}`
    : `${first}, ${second} and ${names.length - 2} more`
}

const aggregateShort = (aggregate: AggregateState): string => {
  if (aggregate.total === 0) {
    return "no bulbs"
  }
  if (aggregate.reachable === 0) {
    return aggregate.failed > 0 ? `${aggregate.failed} unreachable` : "reading…"
  }
  const tail =
    aggregate.failed > 0
      ? `${aggregate.failed} unreachable`
      : `${aggregate.total} ${aggregate.total === 1 ? "bulb" : "bulbs"}`
  return `${aggregate.power} · ${tail}`
}

const aggregateSummary = (aggregate: AggregateState): string => {
  if (aggregate.reachable === 0) {
    return aggregate.failed > 0 ? "No bulb answered" : "Reading the bulbs…"
  }
  if (aggregate.power === "off") {
    return "Off"
  }
  const parts: string[] = [aggregate.power]
  if (aggregate.brightness !== null) {
    parts.push(spanText(aggregate.brightness, "%"))
  }
  if (aggregate.temperature !== null) {
    parts.push(spanText(aggregate.temperature, "K"))
  }
  if (aggregate.color?.kind === "equal") {
    parts.push(aggregate.color.hex)
  }
  return Arr.join(parts, " · ")
}

const noticeDuration = (message: string): number => {
  const words = Math.ceil(message.length / 5)
  return clamp(2500 + words * 220, 3000, 7000)
}

export {
  aggregateColorWord,
  aggregateGlyph,
  aggregateShort,
  aggregateSummary,
  clamp,
  meterUnits,
  nameList,
  noticeDuration,
  percentile,
  powerGlyph,
  powerWord,
  shortState,
  spanText,
  stateSummary,
  truncate,
}
