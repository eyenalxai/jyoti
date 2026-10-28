import type { Effect } from "effect"

import { Result, Schema, String as Str } from "effect"

import type { BulbsModel } from "@/components/bulb/use-bulbs"
import type { BulbsError } from "@/lib/bulbs/error"
import type { Bulb, BulbId, BulbState } from "@/lib/bulbs/schema"
import type { Timer } from "@/lib/timers"

import {
  BRIGHTNESS_MAX,
  BRIGHTNESS_MIN,
  ColorHexSchema,
  TEMPERATURE_MAX,
  TEMPERATURE_MIN,
} from "@/lib/bulbs/schema"
import { Bulbs } from "@/lib/bulbs/service"
import { clamp } from "@/lib/format"
import { run } from "@/lib/runtime"
import { after, cancel } from "@/lib/timers"

interface UseControlsOptions {
  readonly model: BulbsModel
  readonly onError: (message: string) => void
}

interface ControlsModel {
  readonly togglePower: () => Promise<string | null>
  readonly adjustBrightness: (delta: number) => void
  readonly adjustTemperature: (delta: number) => void
  readonly setColor: (hex: string) => Promise<string | null>
  readonly flushPending: () => void
}

interface Pending {
  readonly timer: Timer
  readonly send: () => void
}

const useControls = (options: UseControlsOptions): ControlsModel => {
  const pending = new Map<string, Pending>()

  const schedule = (id: BulbId, control: string, send: () => void) => {
    const key = `${id}:${control}`
    const previous = pending.get(key)
    if (previous !== undefined) {
      cancel(previous.timer)
    }
    const timer = after(180, () => {
      pending.delete(key)
      send()
    })
    pending.set(key, { timer, send })
  }

  const sendControl = (id: BulbId, effect: Effect.Effect<void, BulbsError, Bulbs>) => {
    void run(effect).then((result) => {
      if (result.ok) {
        return null
      }
      options.onError(result.message)
      void options.model.readOne(id)
      return null
    })
  }

  const flushPending = () => {
    for (const entry of pending.values()) {
      cancel(entry.timer)
      entry.send()
    }
    pending.clear()
  }

  const commitPower = (bulb: Bulb, current: BulbState): Promise<string | null> => {
    const target = current.power === false
    options.model.patchState(bulb.id, { power: target })
    return run(Bulbs.use((service) => service.setPower(bulb.id, target))).then((result) => {
      if (result.ok) {
        return null
      }
      options.onError(result.message)
      void options.model.readOne(bulb.id)
      return result.message
    })
  }

  const togglePower = (): Promise<string | null> => {
    const bulb = options.model.selected()
    if (bulb === null) {
      return Promise.resolve(null)
    }
    const state = options.model.states().get(bulb.id)
    if (state !== undefined && state.power !== null) {
      return commitPower(bulb, state)
    }

    return options.model.readOne(bulb.id).then((message) => {
      if (message !== null) {
        options.onError(message)
        return message
      }
      const fresh = options.model.states().get(bulb.id)
      if (fresh === undefined || fresh.power === null) {
        const unknown = "The power state is unknown. Press r to retry."
        options.onError(unknown)
        return unknown
      }
      return commitPower(bulb, fresh)
    })
  }

  const adjustBrightness = (delta: number) => {
    const bulb = options.model.selected()
    if (bulb === null) {
      return
    }
    const state = options.model.states().get(bulb.id)
    if (state === undefined || state.brightness === null) {
      options.onError("Brightness is unknown. Press r to read the bulb.")
      return
    }
    const next = clamp(state.brightness + delta, BRIGHTNESS_MIN, BRIGHTNESS_MAX)
    if (next === state.brightness) {
      return
    }
    options.model.patchState(bulb.id, { brightness: next })
    schedule(bulb.id, "brightness", () => {
      sendControl(
        bulb.id,
        Bulbs.use((service) => service.setBrightness(bulb.id, next)),
      )
    })
  }

  const adjustTemperature = (delta: number) => {
    const bulb = options.model.selected()
    if (bulb === null) {
      return
    }
    const state = options.model.states().get(bulb.id)
    if (state === undefined || state.temperature === null) {
      options.onError("Color temperature is unknown. Press r to read the bulb.")
      return
    }
    const next = clamp(state.temperature + delta, TEMPERATURE_MIN, TEMPERATURE_MAX)
    if (next === state.temperature) {
      return
    }
    options.model.patchState(bulb.id, { temperature: next, mode: "temperature" })
    schedule(bulb.id, "temperature", () => {
      sendControl(
        bulb.id,
        Bulbs.use((service) => service.setTemperature(bulb.id, next)),
      )
    })
  }

  const setColor = (hex: string): Promise<string | null> => {
    const bulb = options.model.selected()
    if (bulb === null) {
      return Promise.resolve(null)
    }
    const normalized = Str.toUpperCase(Str.trim(hex))
    const withHash = Str.startsWith("#")(normalized) ? normalized : `#${normalized}`
    const decoded = Schema.decodeResult(ColorHexSchema)(withHash)
    if (Result.isFailure(decoded)) {
      return Promise.resolve("Enter a color as #RRGGBB.")
    }

    options.model.patchState(bulb.id, { color: decoded.success, mode: "color" })
    return run(Bulbs.use((service) => service.setColor(bulb.id, decoded.success))).then(
      (result) => {
        if (result.ok) {
          return null
        }
        options.onError(result.message)
        void options.model.readOne(bulb.id)
        return result.message
      },
    )
  }

  return {
    togglePower,
    adjustBrightness,
    adjustTemperature,
    setColor,
    flushPending,
  }
}

export { useControls, type ControlsModel }
