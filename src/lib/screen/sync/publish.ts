import { Array as Arr, Effect, Ref, SubscriptionRef } from "effect"

import type { RunDeps, RunHandle } from "@/lib/screen/sync/run"
import type { SyncStatus } from "@/lib/screen/sync/status"

import { isFlashing } from "@/lib/screen/sync/flash"

const patchStatus = Effect.fn("ScreenSync.patchStatus")(function* patchStatus(
  deps: RunDeps,
  patch: Partial<SyncStatus>,
) {
  return yield* SubscriptionRef.update(deps.status, (current) => ({ ...current, ...patch }))
})

const publishLive = Effect.fn("ScreenSync.publishLive")(function* publishLive(
  deps: RunDeps,
  handle: RunHandle,
  patch: Partial<SyncStatus>,
) {
  const progress = yield* Ref.get(handle.progress)
  const guard = yield* Ref.get(handle.flash)
  const synced = Arr.map(handle.bulbs, (bulb) => {
    const current = progress.get(bulb.id)
    return {
      id: bulb.id,
      name: bulb.name,
      status: current?.status ?? "unreachable",
    }
  })
  const reachable = Arr.filter(synced, (bulb) => bulb.status === "syncing").length
  let rejection: string | null = null
  for (const entry of progress.values()) {
    if (entry.rejectedMessage !== null) {
      rejection = entry.rejectedMessage
      break
    }
  }
  yield* SubscriptionRef.update(deps.status, (current) => ({
    ...current,
    ...patch,
    flashing: isFlashing(guard, performance.now()),
    rejection,
    bulbs: synced,
    reachable,
    total: synced.length,
  }))
})

export { patchStatus, publishLive }
