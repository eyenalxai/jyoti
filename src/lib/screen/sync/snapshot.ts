import { Duration, Effect, Result } from "effect"

import type { Bulb, BulbId, BulbState } from "@/lib/bulbs/schema"
import type { BulbsShape } from "@/lib/bulbs/service"
import type { MiioSession } from "@/lib/miio/session"

import { SNAPSHOT_TIMEOUT_MS } from "@/lib/screen/sync/timing"

interface SnapshotResult {
  readonly sessions: ReadonlyMap<BulbId, MiioSession>
  readonly states: ReadonlyMap<BulbId, BulbState>
  readonly reachable: ReadonlySet<BulbId>
}

const snapshotBulbs = Effect.fn("ScreenSync.snapshotBulbs")(function* snapshotBulbs(
  bulbs: BulbsShape,
  listed: readonly Bulb[],
) {
  const opened = yield* Effect.forEach(
    listed,
    (bulb) =>
      bulbs.openSession(bulb.id).pipe(
        Effect.flatMap((session) =>
          session.getState().pipe(Effect.map((state) => ({ session, state }))),
        ),
        Effect.timeout(Duration.millis(SNAPSHOT_TIMEOUT_MS)),
        Effect.result,
        Effect.map((outcome) => ({ bulb, outcome })),
      ),
    { concurrency: "unbounded" },
  )

  const sessions = new Map<BulbId, MiioSession>()
  const states = new Map<BulbId, BulbState>()
  const reachable = new Set<BulbId>()
  for (const entry of opened) {
    if (Result.isSuccess(entry.outcome)) {
      sessions.set(entry.bulb.id, entry.outcome.success.session)
      states.set(entry.bulb.id, entry.outcome.success.state)
      reachable.add(entry.bulb.id)
    }
  }

  return { sessions, states, reachable } satisfies SnapshotResult
})

export { snapshotBulbs, type SnapshotResult }
