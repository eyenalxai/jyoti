import type { Effect } from "effect"

import { Array as Arr } from "effect"

import type { ColorOutcome } from "@/components/bulb/color/outcome"
import type { BulbsModel } from "@/components/bulb/use-bulbs"
import type { ValueSpan } from "@/lib/bulbs/aggregate"
import type { BulbsError } from "@/lib/bulbs/error"
import type { Bulb, BulbId, BulbState } from "@/lib/bulbs/schema"
import type { BulbsShape } from "@/lib/bulbs/service"

import { absoluteStep, aggregateState } from "@/lib/bulbs/aggregate"
import { failuresOf, runBatch } from "@/lib/bulbs/batch"
import {
  BRIGHTNESS_MAX,
  BRIGHTNESS_MIN,
  normalizeColor,
  NEUTRAL_TEMPERATURE,
  TEMPERATURE_MAX,
  TEMPERATURE_MIN,
} from "@/lib/bulbs/schema"
import { Bulbs } from "@/lib/bulbs/service"
import { nameList } from "@/lib/format"
import { run } from "@/lib/runtime"

interface GroupControlsModel {
  readonly togglePower: () => Promise<string | null>
  readonly setWhite: () => Promise<string | null>
  readonly adjustBrightness: (delta: number) => void
  readonly adjustTemperature: (delta: number) => void
  readonly setColor: (hex: string) => Promise<ColorOutcome>
}

type Schedule = (id: BulbId, control: string, send: () => void) => void

interface UseGroupControlsOptions {
  readonly model: BulbsModel
  readonly onError: (message: string) => void
  readonly schedule: Schedule
}

interface Batch {
  readonly failures: string[]
  total: number
  changed: number
  inflight: number
}

const useGroupControls = (options: UseGroupControlsOptions): GroupControlsModel => {
  const flushBatch = (batch: Batch) => {
    if (batch.failures.length === 0) {
      return
    }
    options.onError(
      `${nameList(batch.failures)} failed · ${batch.changed} of ${batch.total} changed`,
    )
  }

  const sendOne = (
    batch: Batch,
    bulb: Bulb,
    stale: boolean,
    effect: Effect.Effect<void, BulbsError, Bulbs>,
  ) => {
    batch.inflight += 1
    void run(effect).then((result) => {
      batch.total += 1
      if (result.ok) {
        batch.changed += 1
      } else {
        batch.failures.push(bulb.name)
      }
      if (!result.ok || stale) {
        void options.model.readOne(bulb.id)
      }
      batch.inflight -= 1
      if (batch.inflight === 0) {
        flushBatch(batch)
      }
      return null
    })
  }

  const togglePower = (): Promise<string | null> => {
    const bulbs = options.model.bulbs()
    if (bulbs.length === 0) {
      return Promise.resolve(null)
    }
    const states = options.model.states()
    const known = Arr.flatMap(bulbs, (bulb) => {
      const state = states.get(bulb.id)
      return state === undefined || state.power === null ? [] : [state.power]
    })
    const target = !(known.length > 0 && Arr.every(known, Boolean))

    for (const bulb of bulbs) {
      options.model.patchState(bulb.id, { power: target })
    }

    return runBatch(bulbs, (service, bulb) => service.setPower(bulb.id, target)).then((result) => {
      if (!result.ok) {
        options.onError(result.message)
        for (const bulb of bulbs) {
          void options.model.readOne(bulb.id)
        }
        return result.message
      }

      const failures = failuresOf(result.entries)
      for (const entry of result.entries) {
        if (!entry.ok || !states.has(entry.bulb.id)) {
          void options.model.readOne(entry.bulb.id)
        }
      }
      if (failures.length === 0) {
        return null
      }

      const names = nameList(Arr.map(failures, (entry) => entry.bulb.name))
      const message = `${names} failed · ${result.entries.length - failures.length} of ${result.entries.length} changed`
      options.onError(message)
      return message
    })
  }

  const setWhite = (): Promise<string | null> => {
    const bulbs = options.model.bulbs()
    if (bulbs.length === 0) {
      return Promise.resolve(null)
    }
    const states = options.model.states()
    const kelvinById = new Map(
      Arr.map(
        bulbs,
        (bulb) => [bulb.id, states.get(bulb.id)?.temperature ?? NEUTRAL_TEMPERATURE] as const,
      ),
    )
    for (const bulb of bulbs) {
      options.model.patchState(bulb.id, {
        temperature: kelvinById.get(bulb.id) ?? NEUTRAL_TEMPERATURE,
        mode: "temperature",
      })
    }

    return runBatch(bulbs, (service, bulb) =>
      service.setTemperature(bulb.id, kelvinById.get(bulb.id) ?? NEUTRAL_TEMPERATURE),
    ).then((result) => {
      if (!result.ok) {
        options.onError(result.message)
        for (const bulb of bulbs) {
          void options.model.readOne(bulb.id)
        }
        return result.message
      }

      const failures = failuresOf(result.entries)
      for (const entry of result.entries) {
        if (!entry.ok || !states.has(entry.bulb.id)) {
          void options.model.readOne(entry.bulb.id)
        }
      }
      if (failures.length === 0) {
        return null
      }

      const names = nameList(Arr.map(failures, (entry) => entry.bulb.name))
      const message = `${names} failed · ${result.entries.length - failures.length} of ${result.entries.length} changed`
      options.onError(message)
      return message
    })
  }

  const adjustAll = (
    control: "brightness" | "temperature",
    delta: number,
    span: ValueSpan | null,
    range: { readonly min: number; readonly max: number },
    unknown: string,
    patch: (value: number) => Partial<BulbState>,
    send: (service: BulbsShape, id: BulbId, value: number) => Effect.Effect<void, BulbsError>,
  ) => {
    if (span === null) {
      options.onError(unknown)
      return
    }
    const next = absoluteStep(span, delta, range.min, range.max)
    if (span.low === span.high && next === span.low) {
      return
    }
    const states = options.model.states()
    const batch: Batch = { failures: [], total: 0, changed: 0, inflight: 0 }
    for (const bulb of options.model.bulbs()) {
      const stale = states.get(bulb.id) === undefined
      options.model.patchState(bulb.id, patch(next))
      options.schedule(bulb.id, control, () => {
        sendOne(
          batch,
          bulb,
          stale,
          Bulbs.use((service) => send(service, bulb.id, next)),
        )
      })
    }
  }

  const adjustBrightness = (delta: number) => {
    const aggregate = aggregateState(
      options.model.bulbs(),
      options.model.states(),
      options.model.stateErrors(),
    )
    adjustAll(
      "brightness",
      delta,
      aggregate.brightness,
      { min: BRIGHTNESS_MIN, max: BRIGHTNESS_MAX },
      "Brightness is unknown. Press r to read the bulbs.",
      (brightness) => ({ brightness }),
      (service, id, value) => service.setBrightness(id, value),
    )
  }

  const adjustTemperature = (delta: number) => {
    const aggregate = aggregateState(
      options.model.bulbs(),
      options.model.states(),
      options.model.stateErrors(),
    )
    adjustAll(
      "temperature",
      delta,
      aggregate.temperature,
      { min: TEMPERATURE_MIN, max: TEMPERATURE_MAX },
      "Color temperature is unknown. Press r to read the bulbs.",
      (temperature) => ({ temperature, mode: "temperature" }),
      (service, id, value) => service.setTemperature(id, value),
    )
  }

  const setColor = (hex: string): Promise<ColorOutcome> => {
    const color = normalizeColor(hex)
    if (color === null) {
      return Promise.resolve({ error: "Enter a color as #RRGGBB.", partial: false })
    }

    const bulbs = options.model.bulbs()
    const states = options.model.states()
    for (const bulb of bulbs) {
      options.model.patchState(bulb.id, { color, mode: "color" })
    }

    return runBatch(bulbs, (service, bulb) => service.setColor(bulb.id, color)).then((result) => {
      if (!result.ok) {
        options.onError(result.message)
        for (const bulb of bulbs) {
          void options.model.readOne(bulb.id)
        }
        return { error: result.message, partial: false }
      }

      const failures = failuresOf(result.entries)
      for (const entry of result.entries) {
        if (!entry.ok || !states.has(entry.bulb.id)) {
          void options.model.readOne(entry.bulb.id)
        }
      }
      if (failures.length === 0) {
        return { error: null, partial: false }
      }

      const names = nameList(Arr.map(failures, (entry) => entry.bulb.name))
      const message = `${names} failed · ${result.entries.length - failures.length} of ${result.entries.length} changed`
      options.onError(message)
      return { error: message, partial: failures.length < result.entries.length }
    })
  }

  return { togglePower, setWhite, adjustBrightness, adjustTemperature, setColor }
}

export { useGroupControls, type GroupControlsModel }
