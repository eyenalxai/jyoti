import { Array as Arr, Context, Effect, Layer, Option } from "effect"

import type { BulbEndpoint, BulbState, MiioReply, Rgb } from "@/lib/bulbs/schema"
import type { MiioTransportShape } from "@/lib/miio/transport"

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
  packColor,
} from "@/lib/bulbs/schema"
import { MiioError } from "@/lib/miio/error"
import { MiioTransport } from "@/lib/miio/transport"

interface MiioShape {
  readonly getState: (endpoint: BulbEndpoint) => Effect.Effect<BulbState, MiioError>
  readonly setPower: (endpoint: BulbEndpoint, on: boolean) => Effect.Effect<void, MiioError>
  readonly setBrightness: (endpoint: BulbEndpoint, value: number) => Effect.Effect<void, MiioError>
  readonly setTemperature: (
    endpoint: BulbEndpoint,
    kelvin: number,
  ) => Effect.Effect<void, MiioError>
  readonly setColor: (endpoint: BulbEndpoint, rgb: Rgb) => Effect.Effect<void, MiioError>
}

interface PropertyParam {
  readonly did: string
  readonly siid: number
  readonly piid: number
  readonly value?: unknown
}

const properties = (piids: readonly number[]): readonly PropertyParam[] =>
  Arr.map(piids, (piid) => ({
    did: `${SIID_LIGHT}-${piid}`,
    siid: SIID_LIGHT,
    piid,
  }))

const propertiesWithValue = (piid: number, value: unknown): readonly PropertyParam[] =>
  Arr.map(properties([piid]), (property) => ({ ...property, value }))

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

const applyProperties = (
  transport: MiioTransportShape,
  endpoint: BulbEndpoint,
  params: readonly PropertyParam[],
) =>
  Effect.gen(function* setProperties() {
    const reply = yield* transport.transact(endpoint, "set_properties", params)
    for (const item of reply.result ?? []) {
      if (item.code !== undefined && item.code !== 0) {
        return yield* new MiioError({
          kind: "device",
          address: endpoint.address,
          message: `Property ${item.siid}-${item.piid} failed with code ${item.code}.`,
        })
      }
    }
    return yield* Effect.void
  })

class Miio extends Context.Service<Miio, MiioShape>()("jyoti/lib/miio/service/Miio") {
  static readonly layer = Layer.effect(
    Miio,
    Effect.gen(function* layer() {
      const transport = yield* MiioTransport

      const getState = Effect.fn("Miio.getState")(function* getState(endpoint: BulbEndpoint) {
        const reply = yield* transport.transact(
          endpoint,
          "get_properties",
          properties([PIID_POWER, PIID_BRIGHTNESS, PIID_TEMPERATURE, PIID_COLOR, PIID_COLOR_MODE]),
        )

        return {
          power: asBoolean(readValue(reply, PIID_POWER)),
          brightness: asInteger(readValue(reply, PIID_BRIGHTNESS), BRIGHTNESS_MIN, BRIGHTNESS_MAX),
          temperature: asInteger(
            readValue(reply, PIID_TEMPERATURE),
            TEMPERATURE_MIN,
            TEMPERATURE_MAX,
          ),
          color: asColor(readValue(reply, PIID_COLOR)),
          mode: asMode(readValue(reply, PIID_COLOR_MODE)),
        } satisfies BulbState
      })

      const setPower = Effect.fn("Miio.setPower")(function* setPower(
        endpoint: BulbEndpoint,
        on: boolean,
      ) {
        yield* applyProperties(transport, endpoint, propertiesWithValue(PIID_POWER, on))
      })

      const setBrightness = Effect.fn("Miio.setBrightness")(function* setBrightness(
        endpoint: BulbEndpoint,
        value: number,
      ) {
        if (!Number.isInteger(value) || value < BRIGHTNESS_MIN || value > BRIGHTNESS_MAX) {
          return yield* new MiioError({
            kind: "value",
            address: endpoint.address,
            message: `Brightness must be ${BRIGHTNESS_MIN}–${BRIGHTNESS_MAX}.`,
          })
        }
        yield* applyProperties(transport, endpoint, propertiesWithValue(PIID_BRIGHTNESS, value))
        return yield* Effect.void
      })

      const setTemperature = Effect.fn("Miio.setTemperature")(function* setTemperature(
        endpoint: BulbEndpoint,
        kelvin: number,
      ) {
        if (!Number.isInteger(kelvin) || kelvin < TEMPERATURE_MIN || kelvin > TEMPERATURE_MAX) {
          return yield* new MiioError({
            kind: "value",
            address: endpoint.address,
            message: `Color temperature must be ${TEMPERATURE_MIN}–${TEMPERATURE_MAX} K.`,
          })
        }
        yield* applyProperties(transport, endpoint, propertiesWithValue(PIID_TEMPERATURE, kelvin))
        return yield* Effect.void
      })

      const setColor = Effect.fn("Miio.setColor")(function* setColor(
        endpoint: BulbEndpoint,
        rgb: Rgb,
      ) {
        const channels = [rgb.red, rgb.green, rgb.blue]
        if (
          !Arr.every(
            channels,
            (channel) => Number.isInteger(channel) && channel >= 0 && channel <= 255,
          )
        ) {
          return yield* new MiioError({
            kind: "value",
            address: endpoint.address,
            message: "Each color channel must be 0–255.",
          })
        }
        yield* applyProperties(transport, endpoint, propertiesWithValue(PIID_COLOR, packColor(rgb)))
        return yield* Effect.void
      })

      return Miio.of({ getState, setPower, setBrightness, setTemperature, setColor })
    }),
  )
}

export { Miio, type MiioShape }
