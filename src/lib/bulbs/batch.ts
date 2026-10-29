import { Array as Arr, Effect, Result } from "effect"

import type { BulbsError } from "@/lib/bulbs/error"
import type { Bulb } from "@/lib/bulbs/schema"
import type { BulbsShape } from "@/lib/bulbs/service"

import { Bulbs } from "@/lib/bulbs/service"
import { run } from "@/lib/runtime"

interface BatchEntry {
  readonly bulb: Bulb
  readonly ok: boolean
}

type BatchOutcome =
  | { readonly ok: true; readonly entries: readonly BatchEntry[] }
  | { readonly ok: false; readonly message: string }

const runBatch = (
  bulbs: readonly Bulb[],
  action: (service: BulbsShape, bulb: Bulb) => Effect.Effect<void, BulbsError>,
): Promise<BatchOutcome> =>
  run(
    Bulbs.use((service) =>
      Effect.forEach(
        bulbs,
        (bulb) =>
          action(service, bulb).pipe(
            Effect.result,
            Effect.map((outcome) => ({ bulb, ok: Result.isSuccess(outcome) })),
          ),
        { concurrency: 4 },
      ),
    ),
  ).then((result) =>
    result.ok ? { ok: true, entries: result.value } : { ok: false, message: result.message },
  )

const failuresOf = (entries: readonly BatchEntry[]): readonly BatchEntry[] =>
  Arr.filter(entries, (entry) => !entry.ok)

export { failuresOf, runBatch }
