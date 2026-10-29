import { Array as Arr, Effect } from "effect"

import type { ColorOutcome } from "@/components/bulb/color/outcome"
import type { BulbsModel } from "@/components/bulb/use-bulbs"
import type { BulbId, BulbState } from "@/lib/bulbs/schema"

import { failuresOf, runBatch } from "@/lib/bulbs/batch"
import { nameList } from "@/lib/format"

interface RestoreContext {
  readonly model: BulbsModel
  readonly onError: (message: string) => void
}

type RestoreValue =
  | { readonly kind: "color"; readonly hex: string }
  | { readonly kind: "temperature"; readonly kelvin: number }

const restoreValue = (state: BulbState): RestoreValue | null => {
  if (state.mode === "temperature" && state.temperature !== null) {
    return { kind: "temperature", kelvin: state.temperature }
  }
  if (state.color !== null) {
    return { kind: "color", hex: state.color }
  }
  if (state.temperature !== null) {
    return { kind: "temperature", kelvin: state.temperature }
  }
  return null
}

const restoreColors = (
  context: RestoreContext,
  snapshot: ReadonlyMap<BulbId, BulbState>,
): Promise<ColorOutcome> => {
  const bulbs = context.model.bulbs()
  const targets = Arr.filter(bulbs, (bulb) => {
    const state = snapshot.get(bulb.id)
    return state !== undefined && restoreValue(state) !== null
  })
  const missing = Arr.filter(bulbs, (bulb) => {
    const state = snapshot.get(bulb.id)
    return state !== undefined && restoreValue(state) === null
  })
  if (missing.length > 0) {
    context.onError(
      `${nameList(Arr.map(missing, (bulb) => bulb.name))} had no previous colour to restore.`,
    )
  }
  if (targets.length === 0) {
    return Promise.resolve({ error: null, partial: false })
  }

  for (const bulb of targets) {
    const state = snapshot.get(bulb.id)
    if (state === undefined) {
      continue
    }
    const restore = restoreValue(state)
    if (restore === null) {
      continue
    }
    if (restore.kind === "color") {
      context.model.patchState(bulb.id, {
        color: state.color,
        temperature: state.temperature,
        mode: "color",
      })
    } else {
      context.model.patchState(bulb.id, {
        color: state.color,
        temperature: state.temperature,
        mode: "temperature",
      })
    }
  }

  return runBatch(targets, (service, bulb) => {
    const state = snapshot.get(bulb.id)
    const restore = state === undefined ? null : restoreValue(state)
    if (restore === null) {
      return Effect.void
    }
    return restore.kind === "color"
      ? service.setColor(bulb.id, restore.hex)
      : service.setTemperature(bulb.id, restore.kelvin)
  }).then((result) => {
    if (!result.ok) {
      context.onError(result.message)
      for (const bulb of targets) {
        void context.model.readOne(bulb.id)
      }
      return { error: result.message, partial: false }
    }

    const failures = failuresOf(result.entries)
    for (const entry of result.entries) {
      if (!entry.ok || !context.model.states().has(entry.bulb.id)) {
        void context.model.readOne(entry.bulb.id)
      }
    }
    if (failures.length === 0) {
      return { error: null, partial: false }
    }

    const names = nameList(Arr.map(failures, (entry) => entry.bulb.name))
    const message = `${names} failed · ${result.entries.length - failures.length} of ${result.entries.length} restored`
    context.onError(message)
    return { error: message, partial: failures.length < result.entries.length }
  })
}

export { restoreColors }
