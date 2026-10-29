import { Array as Arr, Effect } from "effect"

import type { ColorOutcome } from "@/components/bulb/color/outcome"
import type { ColorSnapshot } from "@/components/bulb/color/snapshot"
import type { BulbsModel } from "@/components/bulb/use-bulbs"
import type { RestoreControl } from "@/lib/bulbs/restore"
import type { Bulb, BulbState } from "@/lib/bulbs/schema"

import { failuresOf, runBatch } from "@/lib/bulbs/batch"
import { restoreControl } from "@/lib/bulbs/restore"
import { nameList } from "@/lib/format"

interface RevertContext {
  readonly model: BulbsModel
  readonly onError: (message: string) => void
}

interface RevertPlan {
  readonly bulb: Bulb
  readonly state: BulbState
  readonly control: RestoreControl
}

const revertColors = (context: RevertContext, snapshot: ColorSnapshot): Promise<ColorOutcome> => {
  const plans: RevertPlan[] = []
  const missing: string[] = []

  for (const bulb of context.model.bulbs()) {
    const state = snapshot.get(bulb.id)
    if (state === null) {
      missing.push(bulb.name)
      continue
    }
    if (state === undefined) {
      continue
    }
    const control = restoreControl(state)
    if (control === null) {
      missing.push(bulb.name)
      continue
    }
    plans.push({ bulb, state, control })
  }

  if (missing.length > 0) {
    context.onError(`${nameList(missing)} had no previous colour to revert.`)
  }
  if (plans.length === 0) {
    return Promise.resolve({ error: null, partial: false })
  }

  const byId = new Map(Arr.map(plans, (plan) => [plan.bulb.id, plan] as const))
  for (const plan of plans) {
    context.model.patchState(plan.bulb.id, { ...plan.state, mode: plan.control.kind })
  }

  return runBatch(
    Arr.map(plans, (plan) => plan.bulb),
    (service, bulb) => {
      const plan = byId.get(bulb.id)
      if (plan === undefined) {
        return Effect.void
      }
      return plan.control.kind === "color"
        ? service.setColor(bulb.id, plan.control.hex)
        : service.setTemperature(bulb.id, plan.control.kelvin)
    },
  ).then((result) => {
    if (!result.ok) {
      context.onError(result.message)
      for (const plan of plans) {
        void context.model.readOne(plan.bulb.id)
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
