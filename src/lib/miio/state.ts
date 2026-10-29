import { Array as Arr, Option } from "effect"

import type { BulbState, MiioReply } from "@/lib/bulbs/schema"
import type { PropertyParam } from "@/lib/miio/properties"

import {
  BRIGHTNESS_MAX,
  BRIGHTNESS_MIN,
  PIID_BRIGHTNESS,
  PIID_COLOR,
  PIID_COLOR_MODE,
  PIID_POWER,
  PIID_TEMPERATURE,
  SIID_LIGHT,
  TEMPERATURE_MAX,
  TEMPERATURE_MIN,
  formatColor,
} from "@/lib/bulbs/schema"
import { properties } from "@/lib/miio/properties"

const STATE_PIIDS = [PIID_POWER, PIID_BRIGHTNESS, PIID_TEMPERATURE, PIID_COLOR, PIID_COLOR_MODE]

const readValue = (reply: MiioReply, piid: number): unknown => {
  const item = Option.getOrUndefined(
    Arr.findFirst(reply.result ?? [], (entry) => entry.siid === SIID_LIGHT && entry.piid === piid),
  )
  if (item === undefined) {
    return undefined
  }
  if (item.code !== undefined && item.code !== 0) {
    return undefined
  }
  return item.value
}

const asBoolean = (value: unknown): boolean | null => {
  if (value === true) {
    return true
  }
  if (value === false) {
    return false
  }
  return null
}

const asInteger = (value: unknown, min: number, max: number): number | null => {
  if (typeof value !== "number" || !Number.isInteger(value)) {
    return null
  }
  if (value < min || value > max) {
    return null
  }
  return value
}

const asColor = (value: unknown) =>
  typeof value === "number" && Number.isInteger(value) && value >= 0 && value <= 16_777_215
    ? formatColor(value)
    : null

const asMode = (value: unknown): BulbState["mode"] => {
  if (value === 1) {
    return "color"
  }
  if (value === 2) {
    return "temperature"
  }
  return "unknown"
}

const stateProperties = (): readonly PropertyParam[] => properties(STATE_PIIDS)

const stateFromReply = (reply: MiioReply): BulbState => ({
  power: asBoolean(readValue(reply, PIID_POWER)),
  brightness: asInteger(readValue(reply, PIID_BRIGHTNESS), BRIGHTNESS_MIN, BRIGHTNESS_MAX),
  temperature: asInteger(readValue(reply, PIID_TEMPERATURE), TEMPERATURE_MIN, TEMPERATURE_MAX),
  color: asColor(readValue(reply, PIID_COLOR)),
  mode: asMode(readValue(reply, PIID_COLOR_MODE)),
})

export { stateFromReply, stateProperties }
