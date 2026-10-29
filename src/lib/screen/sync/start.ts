import {
  Array as Arr,
  Cause,
  Deferred,
  Effect,
  Exit,
  Fiber,
  Option,
  Ref,
  Result,
  SubscriptionRef,
} from "effect"

import type { BulbId } from "@/lib/bulbs/schema"
import type { BulbsShape } from "@/lib/bulbs/service"
import type { BulbProgress, BulbSession } from "@/lib/screen/sync/progress"
import type { RunDeps, RunHandle, RunState } from "@/lib/screen/sync/run"
import type { SyncRun } from "@/lib/screen/sync/status"

import { ScreenSyncError } from "@/lib/screen/error"
import { DEFAULT_GRID } from "@/lib/screen/frame"
import { createReducerState } from "@/lib/screen/reducer"
import { bulbFailure, captureFailure } from "@/lib/screen/sync/failure"
import { initialProgress } from "@/lib/screen/sync/progress"
import { idleState, publishLive, runLoop } from "@/lib/screen/sync/run"
import { idleSyncStatus, pickDefaultMonitor, selfCaptureFor } from "@/lib/screen/sync/status"

interface StartContext extends RunDeps {
  readonly bulbs: BulbsShape
  readonly runState: Ref.Ref<RunState>
  readonly pendingMonitor: Ref.Ref<string | null>
}

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

  yield* SubscriptionRef.set(context.status, { ...idleSyncStatus, phase: "starting" as const })

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
      const monitor =
        manual === null
          ? pickDefaultMonitor(monitors)
          : Option.getOrNull(Arr.findFirst(monitors, (entry) => entry.name === manual))
      if (monitor === null) {
        return yield* new ScreenSyncError({
          kind: "no-monitor",
          message:
            manual === null
              ? "No monitor to capture. Start jyoti inside a Hyprland session."
              : `The monitor "${manual}" is no longer available.`,
        })
      }

      const opened = yield* Effect.forEach(
        listed,
        (bulb) =>
          context.bulbs.openSession(bulb.id).pipe(
            Effect.result,
            Effect.map((outcome) => ({ bulb, outcome })),
          ),
        { concurrency: 4 },
      )
      const reachable = new Set<BulbId>()
      const sessions: BulbSession[] = []
      for (const entry of opened) {
        if (Result.isSuccess(entry.outcome)) {
          reachable.add(entry.bulb.id)
          sessions.push({ bulb: entry.bulb, session: entry.outcome.success })
        }
      }

      const handle: RunHandle = {
        bulbs: listed,
        sessions,
        phase: yield* Ref.make<"syncing" | "paused">("syncing"),
        monitor: yield* Ref.make(monitor),
        switching: yield* Ref.make(false),
        reducer: yield* Ref.make(createReducerState()),
        progress: yield* Ref.make<ReadonlyMap<BulbId, BulbProgress>>(
          initialProgress(listed, reachable),
        ),
      }

      yield* Effect.addFinalizer(() => Ref.set(context.runState, idleState))
      yield* Effect.addFinalizer(() => Ref.set(context.pendingMonitor, null))
      yield* Effect.addFinalizer(() => context.capture.stop())
      yield* Effect.addFinalizer(() => SubscriptionRef.set(context.status, idleSyncStatus))
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
                message: "Screen sync start cancelled.",
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
        message: "Screen sync start cancelled.",
      })
    }

    return {
      monitorName: monitor.name,
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
            yield* SubscriptionRef.set(context.status, idleSyncStatus)
          })
        : Effect.void,
    ),
  )
})

export { startRun, type StartContext }
