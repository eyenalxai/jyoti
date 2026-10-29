import { Array as Arr, Cause, Deferred, Effect, Exit, Fiber, Ref, SubscriptionRef } from "effect"

import type { BulbId, BulbState } from "@/lib/bulbs/schema"
import type { MiioSession } from "@/lib/miio/session"
import type { BulbProgress } from "@/lib/screen/sync/progress"
import type { RunDeps, RunHandle, RunState } from "@/lib/screen/sync/run"
import type { SyncRun, SyncStatus } from "@/lib/screen/sync/status"

import { ScreenSyncError } from "@/lib/screen/error"
import { DEFAULT_GRID } from "@/lib/screen/frame"
import { createReducerState } from "@/lib/screen/reducer"
import { SYNC_CANCELLED_MESSAGE } from "@/lib/screen/sync/copy"
import { bulbFailure, captureFailure } from "@/lib/screen/sync/failure"
import { initialFlashGuard } from "@/lib/screen/sync/flash"
import { findMonitor, resolveMonitor, selfCaptureFor } from "@/lib/screen/sync/monitor"
import { switchMonitor } from "@/lib/screen/sync/monitor/switch"
import { initialProgress, setPowers } from "@/lib/screen/sync/progress"
import { patchStatus, publishLive } from "@/lib/screen/sync/publish"
import { idleState, runLoop } from "@/lib/screen/sync/run"
import { snapshotBulbs } from "@/lib/screen/sync/snapshot"
import { idleSyncStatus } from "@/lib/screen/sync/status"

interface StartContext extends RunDeps {
  readonly runState: Ref.Ref<RunState>
  readonly pendingMonitor: Ref.Ref<string | null>
  readonly powerTargets: Ref.Ref<ReadonlyMap<BulbId, boolean | null> | null>
}

const applyPowerTargets = Effect.fn("ScreenSync.applyPowerTargets")(function* applyPowerTargets(
  context: StartContext,
  handle: RunHandle,
) {
  const targets = yield* Ref.modify(
    context.powerTargets,
    (current): readonly [ReadonlyMap<BulbId, boolean | null> | null, null] => [current, null],
  )
  if (
    targets === null ||
    !Arr.some(Arr.fromIterable(targets.values()), (power) => power !== null)
  ) {
    return
  }
  yield* Ref.update(handle.progress, (progress) => setPowers(progress, targets))
})

const startRun = Effect.fn("ScreenSync.startRun")(function* startRun(context: StartContext) {
  const claimed = yield* Ref.modify(context.runState, (current): readonly [boolean, RunState] =>
    current.status === "idle" ? [true, { status: "starting", fiber: null }] : [false, current],
  )
  if (!claimed) {
    return yield* new ScreenSyncError({
      kind: "already-running",
      message: "Screen sync is already running. Press s to stop it.",
    })
  }

  const startingStatus: SyncStatus = { ...idleSyncStatus, phase: "starting" }
  yield* SubscriptionRef.set(context.status, startingStatus)

  return yield* Effect.gen(function* startRunEffect() {
    const ready = yield* Deferred.make<RunHandle, ScreenSyncError>()
    const fiber = yield* Effect.gen(function* run() {
      const listed = yield* context.bulbs.list().pipe(Effect.mapError(bulbFailure))
      if (listed.length === 0) {
        return yield* new ScreenSyncError({
          kind: "no-bulbs",
          message: "Add a bulb before starting screen sync.",
        })
      }

      const monitors = yield* context.capture.listMonitors().pipe(Effect.mapError(captureFailure))
      const manual = yield* Ref.get(context.pendingMonitor)
      const monitor = resolveMonitor(monitors, manual)
      if (monitor === null) {
        yield* Ref.set(context.pendingMonitor, null)
        return yield* new ScreenSyncError({
          kind: "no-monitor",
          message:
            manual === null
              ? "No monitor to capture. Start jyoti inside a Hyprland session."
              : `The monitor "${manual}" is no longer available.`,
        })
      }

      const snapshot = yield* snapshotBulbs(context.bulbs, listed)

      const handle: RunHandle = {
        bulbs: listed,
        monitors,
        sessions: yield* Ref.make<ReadonlyMap<BulbId, MiioSession>>(snapshot.sessions),
        snapshots: yield* Ref.make<ReadonlyMap<BulbId, BulbState>>(snapshot.states),
        armed: yield* Deferred.make<null>(),
        phase: yield* Ref.make<"syncing" | "paused">("syncing"),
        monitor: yield* Ref.make(monitor),
        switching: yield* Ref.make(false),
        reducer: yield* Ref.make(createReducerState()),
        flash: yield* Ref.make(initialFlashGuard()),
        progress: yield* Ref.make<ReadonlyMap<BulbId, BulbProgress>>(
          initialProgress(listed, snapshot.reachable, snapshot.states),
        ),
      }

      yield* applyPowerTargets(context, handle)

      yield* Effect.addFinalizer(() => Ref.set(context.runState, idleState))
      yield* Effect.addFinalizer(() => context.capture.stop())
      yield* Effect.addFinalizer(() =>
        Effect.gen(function* resetStatus() {
          const current = yield* Ref.get(context.runState)
          if (current.status !== "stopping") {
            yield* SubscriptionRef.set(context.status, idleSyncStatus)
          }
        }),
      )
      yield* context.capture
        .start({ monitor: monitor.name, grid: DEFAULT_GRID })
        .pipe(Effect.mapError(captureFailure))

      yield* publishLive(context, handle, {
        phase: "syncing",
        capture: "ok",
        monitor: monitor.name,
        pendingMonitor: null,
        color: null,
        syncBrightness: null,
        frameAgeMs: null,
        selfCapture: selfCaptureFor(monitor, monitors.length === 1),
        error: null,
      })
      yield* Deferred.succeed(ready, handle)
      return yield* runLoop(context, handle)
    }).pipe(
      Effect.catchCause((cause) =>
        Effect.gen(function* reportStartFailure() {
          yield* Deferred.failCause(ready, cause)
          return yield* Effect.failCause(cause)
        }),
      ),
      Effect.scoped,
      Effect.forkDetach,
    )

    yield* Ref.modify(context.runState, (current): readonly [undefined, RunState] =>
      current.status === "starting"
        ? [undefined, { status: "starting", fiber }]
        : [undefined, current],
    )

    const handle = yield* Deferred.await(ready).pipe(
      Effect.catchCause((cause) =>
        Cause.hasInterruptsOnly(cause)
          ? Effect.fail(
              new ScreenSyncError({
                kind: "cancelled",
                message: SYNC_CANCELLED_MESSAGE,
              }),
            )
          : Effect.failCause(cause),
      ),
      Effect.onInterrupt(() => Fiber.interrupt(fiber)),
    )

    const monitor = yield* Ref.get(handle.monitor)
    const claimedRunning = yield* Ref.modify(
      context.runState,
      (current): readonly [boolean, RunState] =>
        current.status === "starting" && current.fiber === fiber
          ? [true, { status: "running", fiber, handle }]
          : [false, current],
    )
    if (!claimedRunning) {
      yield* Fiber.interrupt(fiber)
      return yield* new ScreenSyncError({
        kind: "cancelled",
        message: SYNC_CANCELLED_MESSAGE,
      })
    }

    yield* applyPowerTargets(context, handle)

    const pending = yield* Ref.get(context.pendingMonitor)
    if (pending !== null) {
      const target = findMonitor(handle.monitors, pending)
      if (target !== null && target.name !== monitor.name) {
        yield* switchMonitor(context, handle, target, handle.monitors.length === 1).pipe(
          Effect.ignore,
        )
      }
    }

    yield* Ref.set(context.pendingMonitor, null)
    yield* patchStatus(context, { pendingMonitor: null })

    yield* Deferred.succeed(handle.armed, null)

    const active = yield* Ref.get(handle.monitor)
    return {
      monitorName: active.name,
      bulbCount: handle.bulbs.length,
    } satisfies SyncRun
  }).pipe(
    Effect.onExit((exit) =>
      Exit.isFailure(exit)
        ? Effect.gen(function* resetFailedStart() {
            const current = yield* Ref.get(context.runState)
            if (current.status === "starting" || current.status === "idle") {
              yield* Ref.set(context.runState, idleState)
            }
            yield* Ref.set(context.powerTargets, null)
            const pending = yield* Ref.get(context.pendingMonitor)
            const failedStatus: SyncStatus =
              pending === null ? idleSyncStatus : { ...idleSyncStatus, pendingMonitor: pending }
            yield* SubscriptionRef.set(context.status, failedStatus)
          })
        : Effect.void,
    ),
  )
})

export { startRun, type StartContext }
