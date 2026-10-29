import type { Effect } from "effect"
import type { Accessor } from "solid-js"

import type { ColorOutcome } from "@/components/bulb/color/outcome"
import type { ColorSnapshot } from "@/components/bulb/color/snapshot"
import type { BulbsModel } from "@/components/bulb/use-bulbs"
import type { BulbsError } from "@/lib/bulbs/error"
import type { BulbId } from "@/lib/bulbs/schema"
import type { Timer } from "@/lib/timers"

import { revertColors as revertColorBatch } from "@/components/bulb/color/revert"
import { useGroupControls } from "@/components/bulb/use-group-controls"
import { usePower } from "@/components/bulb/use-power"
import {
  BRIGHTNESS_MAX,
  BRIGHTNESS_MIN,
  NEUTRAL_TEMPERATURE,
  normalizeColor,
  TEMPERATURE_MAX,
  TEMPERATURE_MIN,
} from "@/lib/bulbs/schema"
import { Bulbs } from "@/lib/bulbs/service"
import { clamp } from "@/lib/format"
import { run } from "@/lib/runtime"
import { SYNC_OWNS_CONTROLS } from "@/lib/screen/sync/copy"
import { after, cancel } from "@/lib/timers"

interface UseControlsOptions {
  readonly model: BulbsModel
  readonly onError: (message: string) => void
  readonly syncLocked: Accessor<boolean>
  readonly onBlocked: (message: string) => void
}

interface ControlsModel {
  readonly togglePower: () => Promise<string | null>
  readonly adjustBrightness: (delta: number) => void
  readonly adjustTemperature: (delta: number) => void
  readonly setWhite: () => void
  readonly setColor: (hex: string) => Promise<ColorOutcome>
  readonly revertColors: (snapshot: ColorSnapshot) => Promise<ColorOutcome>
  readonly flushPending: () => void
}

interface Pending {
  readonly timer: Timer
  readonly send: () => void
}

const useControls = (options: UseControlsOptions): ControlsModel => {
  const pending = new Map<string, Pending>()

  const blocked = (): boolean => {
    if (!options.syncLocked()) {
      return false
    }
    options.onBlocked(SYNC_OWNS_CONTROLS)
    return true
  }

  const schedule = (id: BulbId, control: string, send: () => void) => {
    const key = `${id}:${control}`
    const previous = pending.get(key)
    if (previous !== undefined) {
      cancel(previous.timer)
    }
    const timer = after(180, () => {
      pending.delete(key)
      if (options.syncLocked()) {
        return
      }
      send()
    })
    pending.set(key, { timer, send })
  }

  const cancelPending = (id: BulbId, control: string) => {
    const key = `${id}:${control}`
    const entry = pending.get(key)
    if (entry === undefined) {
      return
    }
    cancel(entry.timer)
    pending.delete(key)
  }

  const group = useGroupControls({
    model: options.model,
    onError: options.onError,
    schedule,
  })

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
    if (blocked()) {
      for (const entry of pending.values()) {
        cancel(entry.timer)
      }
      pending.clear()
      return
    }
    for (const entry of pending.values()) {
      cancel(entry.timer)
      entry.send()
    }
    pending.clear()
  }

  const power = usePower({ model: options.model, onError: options.onError, group })

  const adjustBrightness = (delta: number) => {
    if (blocked()) {
      return
    }
    if (options.model.allSelected()) {
      group.adjustBrightness(delta)
      return
    }
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
    if (blocked()) {
      return
    }
    if (options.model.allSelected()) {
      group.adjustTemperature(delta)
      return
    }
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
    cancelPending(bulb.id, "color")
    options.model.patchState(bulb.id, { temperature: next, mode: "temperature" })
    schedule(bulb.id, "temperature", () => {
      sendControl(
        bulb.id,
        Bulbs.use((service) => service.setTemperature(bulb.id, next)),
      )
    })
  }

  const setWhite = () => {
    if (blocked()) {
      return
    }
    if (options.model.allSelected()) {
      void group.setWhite()
      return
    }
    const bulb = options.model.selected()
    if (bulb === null) {
      return
    }
    const state = options.model.states().get(bulb.id)
    const kelvin = state?.temperature ?? NEUTRAL_TEMPERATURE
    cancelPending(bulb.id, "color")
    options.model.patchState(bulb.id, { temperature: kelvin, mode: "temperature" })
    schedule(bulb.id, "temperature", () => {
      sendControl(
        bulb.id,
        Bulbs.use((service) => service.setTemperature(bulb.id, kelvin)),
      )
    })
  }

  const setColor = (hex: string): Promise<ColorOutcome> => {
    if (blocked()) {
      return Promise.resolve({ error: null, partial: false })
    }
    const color = normalizeColor(hex)
    if (color === null) {
      return Promise.resolve({ error: "Enter a color as #RRGGBB.", partial: false })
    }

    if (options.model.allSelected()) {
      for (const bulb of options.model.bulbs()) {
        cancelPending(bulb.id, "temperature")
      }
      return group.setColor(color)
    }

    const bulb = options.model.selected()
    if (bulb === null) {
      return Promise.resolve({ error: null, partial: false })
    }
    const state = options.model.states().get(bulb.id)
    if (state?.color === color && state.mode === "color") {
      return Promise.resolve({ error: null, partial: false })
    }
    cancelPending(bulb.id, "temperature")
    options.model.patchState(bulb.id, { color, mode: "color" })
    return run(Bulbs.use((service) => service.setColor(bulb.id, color))).then((result) => {
      if (result.ok) {
        return { error: null, partial: false }
      }
      options.onError(result.message)
      void options.model.readOne(bulb.id)
      return { error: result.message, partial: false }
    })
  }

  const revertColors = (snapshot: ColorSnapshot): Promise<ColorOutcome> =>
    revertColorBatch({ model: options.model, onError: options.onError }, snapshot)

  return {
    togglePower: power.togglePower,
    adjustBrightness,
    adjustTemperature,
    setWhite,
    setColor,
    revertColors,
    flushPending,
  }
}

export { useControls, type ControlsModel }
