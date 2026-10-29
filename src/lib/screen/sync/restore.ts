import { Duration, Effect, Exit, Ref, Result } from "effect"

import type { Bulb, BulbState } from "@/lib/bulbs/schema"
import type { PropertyParam } from "@/lib/miio/properties"
import type { RunDeps, RunHandle } from "@/lib/screen/sync/run"

import {
  PIID_BRIGHTNESS,
  PIID_COLOR,
  PIID_TEMPERATURE,
  packColor,
  parseColor,
} from "@/lib/bulbs/schema"
import { propertiesWithValue } from "@/lib/miio/properties"
import { markRestored } from "@/lib/screen/sync/progress"
import { publishLive } from "@/lib/screen/sync/publish"
import { RESTORE_TIMEOUT_MS } from "@/lib/screen/sync/timing"

interface RestoreReport {
  readonly restored: number
  readonly notRestored: readonly string[]
  readonly total: number
}

interface RestoreEntry {
  readonly bulb: Bulb
  readonly state: BulbState | undefined
  readonly droveBrightness: boolean
}

const emptyRestoreReport: RestoreReport = { restored: 0, notRestored: [], total: 0 }

const colorParams = (hex: string | null): readonly PropertyParam[] => {
  if (hex === null) {
    return []
  }
  const rgb = parseColor(hex)
  if (rgb === null) {
    return []
  }
  return propertiesWithValue(PIID_COLOR, packColor(rgb))
}

const temperatureParams = (kelvin: number | null): readonly PropertyParam[] =>
  kelvin === null ? [] : propertiesWithValue(PIID_TEMPERATURE, kelvin)

const restoreProperties = (
  state: BulbState,
  droveBrightness: boolean,
): readonly PropertyParam[] | null => {
  const color = colorParams(state.color)
  const temperature = temperatureParams(state.temperature)
  const control =
    state.mode === "temperature"
      ? temperature.length > 0
        ? temperature
        : color
      : color.length > 0
        ? color
        : temperature
  const brightness =
    droveBrightness && state.brightness !== null
      ? propertiesWithValue(PIID_BRIGHTNESS, state.brightness)
      : []
  const params = [...control, ...brightness]
  return params.length === 0 ? null : params
}

const markOutcome = Effect.fn("ScreenSync.markRestoreOutcome")(function* markOutcome(
  deps: RunDeps,
  handle: RunHandle,
  entry: RestoreEntry,
  restored: boolean,
) {
  yield* Ref.update(handle.progress, (current) =>
    markRestored(current, entry.bulb.id, restored ? "restored" : "not-restored"),
  )
  yield* publishLive(deps, handle, {})
})

const restoreBulb = Effect.fn("ScreenSync.restoreBulb")(function* restoreBulb(
  deps: RunDeps,
  handle: RunHandle,
  entry: RestoreEntry,
) {
  const params =
    entry.state === undefined ? null : restoreProperties(entry.state, entry.droveBrightness)
  if (params === null) {
    yield* markOutcome(deps, handle, entry, false)
    return false
  }

  const outcome = yield* Effect.scoped(
    Effect.gen(function* restoreSession() {
      const session = yield* deps.bulbs.openSession(entry.bulb.id)
      yield* session.confirmProperties(params)
    }),
  ).pipe(Effect.timeout(Duration.millis(RESTORE_TIMEOUT_MS)), Effect.result)
  const restored = Result.isSuccess(outcome)
  yield* markOutcome(deps, handle, entry, restored)
  return restored
})

const restoreRun = Effect.fn("ScreenSync.restoreRun")(function* restoreRun(
  deps: RunDeps,
  handle: RunHandle,
) {
  const states = yield* Ref.get(handle.snapshots)
  const progress = yield* Ref.get(handle.progress)
  const outcomes = yield* Effect.forEach(
    handle.bulbs,
    (bulb) =>
      restoreBulb(deps, handle, {
        bulb,
        state: states.get(bulb.id),
        droveBrightness: progress.get(bulb.id)?.droveBrightness ?? false,
      }).pipe(
        Effect.exit,
        Effect.map((exit) => ({ bulb, restored: Exit.isSuccess(exit) && exit.value })),
      ),
    { concurrency: "unbounded" },
  )

  const notRestored: string[] = []
  let restored = 0
  for (const entry of outcomes) {
    if (entry.restored) {
      restored += 1
    } else {
      notRestored.push(entry.bulb.name)
    }
  }
  return {
    restored,
    notRestored,
    total: handle.bulbs.length,
  } satisfies RestoreReport
})

export { emptyRestoreReport, restoreRun, type RestoreReport }
