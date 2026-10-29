import type { Fiber, SubscriptionRef } from "effect"

import { Cause, Deferred, Effect, Option, Ref } from "effect"

import type { Bulb, BulbId, BulbState } from "@/lib/bulbs/schema"
import type { BulbsShape } from "@/lib/bulbs/service"
import type { MiioError } from "@/lib/miio/error"
import type { MiioSession } from "@/lib/miio/session"
import type { ScreenCaptureShape } from "@/lib/screen/capture"
import type { ScreenSyncError } from "@/lib/screen/error"
import type { Monitor } from "@/lib/screen/frame"
import type { DerivedColor, ReducerState } from "@/lib/screen/reducer"
import type { FlashGuard } from "@/lib/screen/sync/flash"
import type { BulbProgress, SendPlan } from "@/lib/screen/sync/progress"
import type { SyncStatus } from "@/lib/screen/sync/status"

import { prepareFrame } from "@/lib/screen/grid"
import { reducePrepared } from "@/lib/screen/reducer"
import { syncBrightnessFor } from "@/lib/screen/sync/brightness"
import { guardFlash } from "@/lib/screen/sync/flash"
import {
  markInflight,
  markProbe,
  markReachable,
  missProgress,
  planSend,
  probeDue,
  propertiesFor,
  settleProgress,
} from "@/lib/screen/sync/progress"
import { publishLive } from "@/lib/screen/sync/publish"
import { LOST_FRAME_MS, PROBE_INTERVAL_MS, STALE_FRAME_MS, TICK_MS } from "@/lib/screen/sync/timing"

interface RunHandle {
  readonly bulbs: readonly Bulb[]
  readonly monitors: readonly Monitor[]
  readonly sessions: Ref.Ref<ReadonlyMap<BulbId, MiioSession>>
  readonly snapshots: Ref.Ref<ReadonlyMap<BulbId, BulbState>>
  readonly armed: Deferred.Deferred<null>
  readonly phase: Ref.Ref<"syncing" | "paused">
  readonly monitor: Ref.Ref<Monitor>
  readonly switching: Ref.Ref<boolean>
  readonly reducer: Ref.Ref<ReducerState>
  readonly flash: Ref.Ref<FlashGuard>
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
  readonly bulbs: BulbsShape
}

const healthPatch = (
  capture: SyncStatus["capture"],
  frameAgeMs: number | null,
): Partial<SyncStatus> => ({ capture, frameAgeMs })

const settle = Effect.fn("ScreenSync.settle")(function* settle(
  deps: RunDeps,
  handle: RunHandle,
  id: BulbId,
  target: DerivedColor,
  plan: SendPlan,
  error: MiioError | null,
) {
  yield* Ref.update(handle.progress, (current) => settleProgress(current, id, target, plan, error))
  yield* publishLive(deps, handle, {})
})

const settleDefect = Effect.fn("ScreenSync.settleDefect")(function* settleDefect(
  deps: RunDeps,
  handle: RunHandle,
  id: BulbId,
) {
  yield* Ref.update(handle.progress, (current) => missProgress(current, id))
  yield* publishLive(deps, handle, {})
})

const sendTarget = Effect.fn("ScreenSync.sendTarget")(function* sendTarget(
  deps: RunDeps,
  handle: RunHandle,
  target: DerivedColor,
  targetBrightness: number,
  flashHold: boolean,
) {
  const progress = yield* Ref.get(handle.progress)
  const sessions = yield* Ref.get(handle.sessions)
  for (const bulb of handle.bulbs) {
    const entry = progress.get(bulb.id)
    const session = sessions.get(bulb.id)
    if (entry === undefined || session === undefined) {
      continue
    }
    const plan = planSend(entry, target, targetBrightness, flashHold)
    if (plan === null) {
      continue
    }
    yield* Ref.update(handle.progress, (current) => markInflight(current, bulb.id))
    yield* Effect.forkScoped(
      session.setProperties(propertiesFor(target, plan)).pipe(
        Effect.matchEffect({
          onFailure: (error) => settle(deps, handle, bulb.id, target, plan, error),
          onSuccess: () => settle(deps, handle, bulb.id, target, plan, null),
        }),
        Effect.catchCauseIf(
          (cause) => !Cause.hasInterruptsOnly(cause),
          () => settleDefect(deps, handle, bulb.id),
        ),
      ),
    )
  }
})

const recoverBulb = Effect.fn("ScreenSync.recoverBulb")(function* recoverBulb(
  deps: RunDeps,
  handle: RunHandle,
  id: BulbId,
) {
  yield* Ref.update(handle.progress, (current) => markReachable(current, id))
  yield* publishLive(deps, handle, {})
})

const probeBulb = Effect.fn("ScreenSync.probeBulb")(function* probeBulb(
  deps: RunDeps,
  handle: RunHandle,
  bulb: Bulb,
) {
  const current = (yield* Ref.get(handle.sessions)).get(bulb.id)
  if (current !== undefined) {
    yield* current.hello()
    return
  }
  const session = yield* deps.bulbs.openSession(bulb.id)
  yield* Ref.update(handle.sessions, (sessions) => new Map(sessions).set(bulb.id, session))
})

const probeUnreachable = Effect.fn("ScreenSync.probeUnreachable")(function* probeUnreachable(
  deps: RunDeps,
  handle: RunHandle,
  now: number,
) {
  const progress = yield* Ref.get(handle.progress)
  for (const bulb of handle.bulbs) {
    const entry = progress.get(bulb.id)
    if (entry === undefined || !probeDue(entry, now)) {
      continue
    }
    yield* Ref.update(handle.progress, (current) =>
      markProbe(current, bulb.id, now + PROBE_INTERVAL_MS),
    )
    yield* Effect.forkScoped(
      probeBulb(deps, handle, bulb).pipe(
        Effect.matchEffect({
          onFailure: () => Effect.void,
          onSuccess: () => recoverBulb(deps, handle, bulb.id),
        }),
        Effect.catchCauseIf(
          (cause) => !Cause.hasInterruptsOnly(cause),
          () => Effect.void,
        ),
      ),
    )
  }
})

const tick = Effect.fn("ScreenSync.tick")(function* tick(
  deps: RunDeps,
  handle: RunHandle,
  elapsedMs: number,
) {
  if (yield* Ref.get(handle.switching)) {
    return
  }

  yield* probeUnreachable(deps, handle, performance.now())

  const latest = yield* deps.capture.latest()
  if (Option.isNone(latest)) {
    yield* publishLive(deps, handle, healthPatch("lost", null))
    return
  }

  const frame = latest.value
  if (frame.ageMs > LOST_FRAME_MS) {
    yield* publishLive(deps, handle, healthPatch("lost", frame.ageMs))
    return
  }
  if (frame.ageMs > STALE_FRAME_MS) {
    yield* publishLive(deps, handle, healthPatch("stale", frame.ageMs))
    return
  }

  const prepared = prepareFrame(frame)
  if (prepared.blackout) {
    yield* publishLive(deps, handle, healthPatch("black", frame.ageMs))
    return
  }

  yield* publishLive(deps, handle, { capture: "ok", frameAgeMs: frame.ageMs })
  if ((yield* Ref.get(handle.phase)) === "paused") {
    return
  }

  const reduction = reducePrepared(prepared, yield* Ref.get(handle.reducer), elapsedMs)
  yield* Ref.set(handle.reducer, reduction.state)

  const target = reduction.derived
  const targetBrightness = syncBrightnessFor(target.sceneL)
  const guarded = guardFlash(yield* Ref.get(handle.flash), targetBrightness, performance.now())
  yield* Ref.set(handle.flash, guarded.guard)
  yield* publishLive(deps, handle, {
    color: target,
    syncBrightness: targetBrightness,
    capture: "ok",
    frameAgeMs: frame.ageMs,
  })
  yield* sendTarget(deps, handle, target, targetBrightness, guarded.flashing)
})

const runLoop = Effect.fn("ScreenSync.runLoop")(function* runLoop(
  deps: RunDeps,
  handle: RunHandle,
) {
  yield* Deferred.await(handle.armed)
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

export { idleState, runLoop }
export type { RunDeps, RunHandle, RunState }
