import { Array as Arr, Effect } from "effect"

import type { ColorOutcome } from "@/components/bulb/color/outcome"
import type { ColorSnapshot } from "@/components/bulb/color/snapshot"
import type { BulbsModel } from "@/components/bulb/use-bulbs"

import { failuresOf, runBatch } from "@/lib/bulbs/batch"
import { restoreControl } from "@/lib/bulbs/restore"
import { nameList } from "@/lib/format"

interface RevertContext {
  readonly model: BulbsModel
  readonly onError: (message: string) => void
}

const revertColors = (context: RevertContext, snapshot: ColorSnapshot): Promise<ColorOutcome> => {
  const bulbs = context.model.bulbs()
  const targets = Arr.filter(bulbs, (bulb) => {
    const state = snapshot.get(bulb.id)
    return state !== null && state !== undefined && restoreControl(state) !== null
  })
  const missing = Arr.filter(bulbs, (bulb) => {
    const state = snapshot.get(bulb.id)
    return state === null || (state !== undefined && restoreControl(state) === null)
  })
  if (missing.length > 0) {
    context.onError(
      `${nameList(Arr.map(missing, (bulb) => bulb.name))} had no previous colour to revert.`,
    )
  }
  if (targets.length === 0) {
    return Promise.resolve({ error: null, partial: false })
  }

  for (const bulb of targets) {
    const state = snapshot.get(bulb.id)
    if (state === null || state === undefined) {
      continue
    }
    const control = restoreControl(state)
    if (control === null) {
      continue
    }
    context.model.patchState(bulb.id, { ...state, mode: control.kind })
  }

  return runBatch(targets, (service, bulb) => {
    const state = snapshot.get(bulb.id)
    const control = state === null || state === undefined ? null : restoreControl(state)
    if (control === null) {
      return Effect.void
    }
    return control.kind === "color"
      ? service.setColor(bulb.id, control.hex)
      : service.setTemperature(bulb.id, control.kelvin)
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
    const message = `${names} failed · ${result.entries.length - failures.length} of ${result.entries.length} reverted`
    context.onError(message)
    return { error: message, partial: failures.length < result.entries.length }
  })
}

export { revertColors }
