import type { Stream } from "effect"

import { Context, Effect, Fiber, Layer, Ref, Semaphore, SubscriptionRef } from "effect"

import type { Monitor } from "@/lib/screen/frame"
import type { RestoreReport } from "@/lib/screen/sync/restore"
import type { RunDeps, RunState } from "@/lib/screen/sync/run"
import type { StartContext } from "@/lib/screen/sync/start"
import type { SyncRun, SyncStatus } from "@/lib/screen/sync/status"

import { Bulbs } from "@/lib/bulbs/service"
import { ScreenCapture } from "@/lib/screen/capture"
import { ScreenSyncError } from "@/lib/screen/error"
import { resetSmoothing } from "@/lib/screen/reducer"
import { captureFailure } from "@/lib/screen/sync/failure"
import { findMonitor } from "@/lib/screen/sync/monitor"
import { switchMonitor } from "@/lib/screen/sync/monitor/switch"
import { emptyRestoreReport, restoreRun } from "@/lib/screen/sync/restore"
import { idleState, patchStatus, publishLive } from "@/lib/screen/sync/run"
import { startRun } from "@/lib/screen/sync/start"
import { idleSyncStatus } from "@/lib/screen/sync/status"

interface ScreenSyncShape {
  readonly status: Stream.Stream<SyncStatus>
  readonly start: () => Effect.Effect<SyncRun, ScreenSyncError>
  readonly pause: () => Effect.Effect<void, ScreenSyncError>
  readonly resume: () => Effect.Effect<void, ScreenSyncError>
  readonly stop: () => Effect.Effect<RestoreReport>
  readonly clearRestore: () => Effect.Effect<void>
  readonly listMonitors: () => Effect.Effect<readonly Monitor[], ScreenSyncError>
  readonly selectMonitor: (name: string) => Effect.Effect<void, ScreenSyncError>
}

class ScreenSyncEngine extends Context.Service<ScreenSyncEngine, ScreenSyncShape>()(
  "jyoti/lib/screen/sync/ScreenSyncEngine",
) {
  static readonly layer = Layer.effect(
    ScreenSyncEngine,
    Effect.gen(function* layer() {
      const bulbs = yield* Bulbs
      const capture = yield* ScreenCapture
      const runState = yield* Ref.make<RunState>(idleState)
      const pendingMonitor = yield* Ref.make<string | null>(null)
      const status = yield* SubscriptionRef.make<SyncStatus>(idleSyncStatus)
      const stopLock = yield* Semaphore.make(1)
      const deps: RunDeps = { capture, status, bulbs }
      const context: StartContext = { capture, status, bulbs, runState, pendingMonitor }

      const start = Effect.fn("ScreenSync.start")(function* start() {
        return yield* startRun(context)
      })

      const pause = Effect.fn("ScreenSync.pause")(function* pause() {
        const current = yield* Ref.get(runState)
        if (current.status !== "running") {
          return yield* new ScreenSyncError({
            kind: "not-running",
            message: "Screen sync is not running.",
          })
        }
        if ((yield* Ref.get(current.handle.phase)) === "paused") {
          return yield* Effect.void
        }
        yield* Ref.set(current.handle.phase, "paused")
        yield* publishLive(deps, current.handle, { phase: "paused" })
        return yield* Effect.void
      })

      const resume = Effect.fn("ScreenSync.resume")(function* resume() {
        const current = yield* Ref.get(runState)
        if (current.status !== "running") {
          return yield* new ScreenSyncError({
            kind: "not-running",
            message: "Screen sync is not running.",
          })
        }
        if ((yield* Ref.get(current.handle.phase)) === "syncing") {
          return yield* Effect.void
        }
        yield* Ref.update(current.handle.reducer, resetSmoothing)
        yield* Ref.set(current.handle.phase, "syncing")
        yield* publishLive(deps, current.handle, { phase: "syncing" })
        return yield* Effect.void
      })

      const stopLocked = Effect.fn("ScreenSync.stopLocked")(function* stopLocked() {
        const current = yield* Ref.get(runState)
        if (current.status === "idle") {
          yield* Ref.set(pendingMonitor, null)
          yield* SubscriptionRef.set(status, idleSyncStatus)
          return emptyRestoreReport
        }

        if (current.status === "starting") {
          if (current.fiber !== null) {
            yield* Fiber.interrupt(current.fiber)
          }
          yield* capture.stop()
          yield* Ref.set(pendingMonitor, null)
          yield* Ref.set(runState, idleState)
          yield* SubscriptionRef.set(status, idleSyncStatus)
          return emptyRestoreReport
        }

        if (current.status === "running") {
          yield* Ref.set(runState, {
            status: "stopping",
            fiber: current.fiber,
            handle: current.handle,
          })
        }
        yield* Fiber.interrupt(current.fiber)
        // The run fiber's finalizer resets the state; hold stopping through the restore.
        yield* Ref.set(runState, {
          status: "stopping",
          fiber: current.fiber,
          handle: current.handle,
        })
        yield* capture.stop()
        yield* publishLive(deps, current.handle, { phase: "stopping" })
        const report = yield* restoreRun(deps, current.handle)
        const final = yield* SubscriptionRef.get(status)
        yield* Ref.set(pendingMonitor, null)
        yield* Ref.set(runState, idleState)
        yield* SubscriptionRef.set(status, {
          ...idleSyncStatus,
          bulbs: final.bulbs,
          total: final.total,
        })
        return report
      })

      const stop = Effect.fn("ScreenSync.stop")(function* stop() {
        return yield* stopLock.withPermit(stopLocked())
      })

      const clearRestore = Effect.fn("ScreenSync.clearRestore")(function* clearRestore() {
        const current = yield* Ref.get(runState)
        if (current.status !== "idle") {
          return yield* Effect.void
        }
        yield* SubscriptionRef.set(status, idleSyncStatus)
        return yield* Effect.void
      })

      const listMonitors = Effect.fn("ScreenSync.listMonitors")(function* listMonitors() {
        return yield* capture.listMonitors().pipe(Effect.mapError(captureFailure))
      })

      const selectMonitor = Effect.fn("ScreenSync.selectMonitor")(function* selectMonitor(
        name: string,
      ) {
        const monitors = yield* capture.listMonitors().pipe(Effect.mapError(captureFailure))
        const monitor = findMonitor(monitors, name)
        if (monitor === null) {
          return yield* new ScreenSyncError({
            kind: "no-monitor",
            message: `The monitor "${name}" is not available.`,
          })
        }

        const current = yield* Ref.get(runState)
        if (current.status === "idle" || current.status === "starting") {
          yield* Ref.set(pendingMonitor, name)
          yield* patchStatus(deps, { pendingMonitor: name })
          return yield* Effect.void
        }
        if (current.status !== "running") {
          return yield* new ScreenSyncError({
            kind: "not-ready",
            message: "Wait for the run to stop before changing monitors.",
          })
        }

        yield* switchMonitor(deps, current.handle, monitor, monitors.length === 1)
        return yield* Effect.void
      })

      return ScreenSyncEngine.of({
        status: SubscriptionRef.changes(status),
        start,
        pause,
        resume,
        stop,
        clearRestore,
        listMonitors,
        selectMonitor,
      })
    }),
  )
}

export { ScreenSyncEngine }
