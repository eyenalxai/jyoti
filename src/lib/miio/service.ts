import { Array as Arr, Context, Effect, Layer } from "effect"

import type { BulbEndpoint, BulbState, Rgb } from "@/lib/bulbs/schema"
import type { PropertyParam } from "@/lib/miio/properties"
import type { MiioTransportShape } from "@/lib/miio/transport"

import {
  BRIGHTNESS_MAX,
  BRIGHTNESS_MIN,
  PIID_BRIGHTNESS,
  PIID_COLOR,
  PIID_POWER,
  PIID_TEMPERATURE,
  TEMPERATURE_MAX,
  TEMPERATURE_MIN,
  packColor,
} from "@/lib/bulbs/schema"
import { MiioError } from "@/lib/miio/error"
import { commandError, propertiesWithValue } from "@/lib/miio/properties"
import { stateFromReply, stateProperties } from "@/lib/miio/state"
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

const applyProperties = (
  transport: MiioTransportShape,
  endpoint: BulbEndpoint,
  params: readonly PropertyParam[],
) =>
  Effect.gen(function* setProperties() {
    const reply = yield* transport.transact(endpoint, "set_properties", params)
    const failure = commandError(reply, endpoint.address)
    if (failure !== null) {
      return yield* failure
    }
    return yield* Effect.void
  })

class Miio extends Context.Service<Miio, MiioShape>()("jyoti/lib/miio/service/Miio") {
  static readonly layer = Layer.effect(
    Miio,
    Effect.gen(function* layer() {
      const transport = yield* MiioTransport

      const getState = Effect.fn("Miio.getState")(function* getState(endpoint: BulbEndpoint) {
        const reply = yield* transport.transact(endpoint, "get_properties", stateProperties())
        return stateFromReply(reply)
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
