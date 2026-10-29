import type { Fiber, Scope } from "effect"

import { Array as Arr, Effect, Option, Ref, SubscriptionRef } from "effect"

import type { Bulb, BulbId } from "@/lib/bulbs/schema"
import type { MiioError } from "@/lib/miio/error"
import type { ScreenCaptureShape } from "@/lib/screen/capture"
import type { ScreenSyncError } from "@/lib/screen/error"
import type { Monitor } from "@/lib/screen/frame"
import type { DerivedColor, ReducerState } from "@/lib/screen/reducer"
import type { BulbProgress, BulbSession } from "@/lib/screen/sync/progress"
import type { SyncStatus } from "@/lib/screen/sync/status"

import { prepareFrame } from "@/lib/screen/grid"
import { reducePrepared, shouldPublish } from "@/lib/screen/reducer"
import {
  markInflight,
  missProgress,
  propertiesFor,
  settleProgress,
} from "@/lib/screen/sync/progress"
import { LOST_FRAME_MS, STALE_FRAME_MS, TICK_MS, syncBrightnessFor } from "@/lib/screen/sync/status"

interface RunHandle {
  readonly bulbs: readonly Bulb[]
  readonly sessions: readonly BulbSession[]
  readonly phase: Ref.Ref<"syncing" | "paused">
  readonly monitor: Ref.Ref<Monitor>
  readonly switching: Ref.Ref<boolean>
  readonly reducer: Ref.Ref<ReducerState>
  readonly progress: Ref.Ref<ReadonlyMap<BulbId, BulbProgress>>
}

type RunState =
  | { readonly status: "idle" }
  | {
      readonly status: "starting"
      readonly fiber: Fiber.Fiber<void, ScreenSyncError> | null
    }
  | {
      readonly status: "running"
      readonly fiber: Fiber.Fiber<void, ScreenSyncError>
      readonly handle: RunHandle
    }
  | {
      readonly status: "stopping"
      readonly fiber: Fiber.Fiber<void, ScreenSyncError>
      readonly handle: RunHandle
    }

const idleState: RunState = { status: "idle" }

interface RunDeps {
  readonly capture: ScreenCaptureShape
  readonly status: SubscriptionRef.SubscriptionRef<SyncStatus>
}

const patchStatus = (deps: RunDeps, patch: Partial<SyncStatus>): Effect.Effect<void> =>
  SubscriptionRef.update(deps.status, (current) => ({ ...current, ...patch }))

const publishLive = (
  deps: RunDeps,
  handle: RunHandle,
  patch: Partial<SyncStatus>,
): Effect.Effect<void> =>
  Effect.gen(function* publishLiveEffect() {
    const progress = yield* Ref.get(handle.progress)
    const synced = Arr.map(handle.bulbs, (bulb) => {
      const current = progress.get(bulb.id)
      return {
        id: bulb.id,
        name: bulb.name,
        status: current?.status ?? "unreachable",
      }
    })
    const reachable = Arr.filter(synced, (bulb) => bulb.status === "syncing").length
    yield* SubscriptionRef.update(deps.status, (current) => ({
      ...current,
      ...patch,
      bulbs: synced,
      reachable,
      total: synced.length,
    }))
  })

const settle = (
  deps: RunDeps,
  handle: RunHandle,
  id: BulbId,
  target: DerivedColor,
  error: MiioError | null,
): Effect.Effect<void> =>
  Effect.gen(function* settleSend() {
    yield* Ref.update(handle.progress, (current) => settleProgress(current, id, target, error))
    if (error !== null && error.kind === "device") {
      yield* patchStatus(deps, { error: error.message })
    } else if (error === null) {
      yield* patchStatus(deps, { error: null })
    }
    yield* publishLive(deps, handle, {})
  })

const settleDefect = (deps: RunDeps, handle: RunHandle, id: BulbId): Effect.Effect<void> =>
  Effect.gen(function* settleUnexpected() {
    yield* Ref.update(handle.progress, (current) => missProgress(current, id))
    yield* publishLive(deps, handle, {})
  })

const sendTarget = (
  deps: RunDeps,
  handle: RunHandle,
  target: DerivedColor,
): Effect.Effect<void, never, Scope.Scope> =>
  Effect.gen(function* sendTargetEffect() {
    const progress = yield* Ref.get(handle.progress)
    for (const item of handle.sessions) {
      const entry = progress.get(item.bulb.id)
      if (entry === undefined || entry.status !== "syncing" || entry.inflight) {
        continue
      }
      if (!shouldPublish(target, entry.last)) {
        continue
      }
      yield* Ref.update(handle.progress, (current) => markInflight(current, item.bulb.id))
      yield* Effect.forkScoped(
        item.session.setProperties(propertiesFor(target)).pipe(
          Effect.matchEffect({
            onFailure: (error) => settle(deps, handle, item.bulb.id, target, error),
            onSuccess: () => settle(deps, handle, item.bulb.id, target, null),
          }),
          Effect.catchCause(() => settleDefect(deps, handle, item.bulb.id)),
        ),
      )
    }
  })

const tick = (
  deps: RunDeps,
  handle: RunHandle,
  elapsedMs: number,
): Effect.Effect<void, never, Scope.Scope> =>
  Effect.gen(function* runTick() {
    if (yield* Ref.get(handle.switching)) {
      return
    }

    const latest = yield* deps.capture.latest()
    if (Option.isNone(latest)) {
      yield* publishLive(deps, handle, { capture: "lost", frameAgeMs: null })
      return
    }

    const frame = latest.value
    if (frame.ageMs > LOST_FRAME_MS) {
      yield* publishLive(deps, handle, { capture: "lost", frameAgeMs: frame.ageMs })
      return
    }
    if (frame.ageMs > STALE_FRAME_MS) {
      yield* publishLive(deps, handle, { capture: "stale", frameAgeMs: frame.ageMs })
      return
    }

    const prepared = prepareFrame(frame)
    if (prepared.blackout) {
      yield* publishLive(deps, handle, { capture: "black", frameAgeMs: frame.ageMs })
      return
    }

    yield* publishLive(deps, handle, { capture: "ok", frameAgeMs: frame.ageMs })
    if ((yield* Ref.get(handle.phase)) === "paused") {
      return
    }

    const reduction = reducePrepared(prepared, yield* Ref.get(handle.reducer), elapsedMs)
    yield* Ref.set(handle.reducer, reduction.state)

    const target = reduction.derived
    yield* publishLive(deps, handle, {
      color: target,
      syncBrightness: syncBrightnessFor(target.sceneL),
      capture: "ok",
      frameAgeMs: frame.ageMs,
    })
    yield* sendTarget(deps, handle, target)
  })

const runLoop = (deps: RunDeps, handle: RunHandle): Effect.Effect<void, never, Scope.Scope> =>
  Effect.gen(function* runLoopEffect() {
    let previousTick = performance.now()
    let deadline = performance.now()

    while (true) {
      const now = performance.now()
      if (now < deadline) {
        yield* Effect.sleep(deadline - now)
      }

      const startedAt = performance.now()
      yield* tick(deps, handle, startedAt - previousTick)
      previousTick = startedAt
      deadline += TICK_MS
      while (deadline <= performance.now()) {
        deadline += TICK_MS
      }
    }
  })

export { idleState, patchStatus, publishLive, runLoop }
export type { RunDeps, RunHandle, RunState }
