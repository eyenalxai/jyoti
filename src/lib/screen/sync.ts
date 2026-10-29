import type { Scope } from "effect"

import { Array as Arr, Context, Deferred, Effect, Exit, Fiber, Layer, Option, Ref } from "effect"

import type { BulbsError } from "@/lib/bulbs/error"
import type { PropertyParam } from "@/lib/miio/properties"
import type { MiioSession } from "@/lib/miio/session"
import type { ScreenCaptureError } from "@/lib/screen/error"
import type { Monitor } from "@/lib/screen/frame"
import type { DerivedColor, ReducerState } from "@/lib/screen/reducer"

import { PIID_COLOR, PIID_TEMPERATURE, packColor, parseColor } from "@/lib/bulbs/schema"
import { Bulbs } from "@/lib/bulbs/service"
import { propertiesWithValue } from "@/lib/miio/properties"
import { ScreenCapture } from "@/lib/screen/capture"
import { ScreenSyncError } from "@/lib/screen/error"
import { DEFAULT_GRID } from "@/lib/screen/frame"
import { prepareFrame } from "@/lib/screen/grid"
import {
  NEUTRAL_KELVIN,
  createReducerState,
  reducePrepared,
  shouldPublish,
} from "@/lib/screen/reducer"

const TICK_MS = 667
const STALE_FRAME_MS = 1000

interface SyncRun {
  readonly bulbName: string
  readonly monitorName: string
}

type RunState =
  | { readonly status: "idle" }
  | { readonly status: "starting" }
  | { readonly status: "running"; readonly fiber: Fiber.Fiber<void, ScreenSyncError> }

const idleState: RunState = { status: "idle" }

interface BulbRunState {
  readonly last: DerivedColor | null
  readonly inflight: boolean
}

interface ScreenSyncShape {
  readonly start: () => Effect.Effect<SyncRun, ScreenSyncError>
  readonly stop: () => Effect.Effect<void>
  readonly running: () => Effect.Effect<boolean>
}

const pickMonitor = (monitors: readonly Monitor[]): Monitor | null => {
  const first = monitors[0]
  if (first === undefined) {
    return null
  }
  if (monitors.length === 1) {
    return first
  }
  return Option.getOrElse(
    Arr.findFirst(monitors, (monitor) => monitor.focused === false),
    () => first,
  )
}

const propertiesFor = (target: DerivedColor): readonly PropertyParam[] => {
  if (target.mode === "temperature") {
    return propertiesWithValue(PIID_TEMPERATURE, target.kelvin ?? NEUTRAL_KELVIN)
  }
  const rgb = parseColor(target.hex)
  if (rgb === null) {
    return propertiesWithValue(PIID_TEMPERATURE, NEUTRAL_KELVIN)
  }
  return propertiesWithValue(PIID_COLOR, packColor(rgb))
}

const captureFailure = (error: ScreenCaptureError): ScreenSyncError =>
  new ScreenSyncError({ kind: "capture", message: error.message, cause: error })

const bulbFailure = (error: BulbsError): ScreenSyncError =>
  new ScreenSyncError({ kind: "bulb", message: error.message, cause: error })

class ScreenSyncEngine extends Context.Service<ScreenSyncEngine, ScreenSyncShape>()(
  "jyoti/lib/screen/sync/ScreenSyncEngine",
) {
  static readonly layer = Layer.effect(
    ScreenSyncEngine,
    Effect.gen(function* layer() {
      const bulbs = yield* Bulbs
      const capture = yield* ScreenCapture
      const runState = yield* Ref.make<RunState>(idleState)

      const tick = (
        session: MiioSession,
        reducer: Ref.Ref<ReducerState>,
        state: Ref.Ref<BulbRunState>,
        elapsedMs: number,
      ): Effect.Effect<void, never, Scope.Scope> =>
        Effect.gen(function* runTick() {
          const latest = yield* capture.latest()
          if (Option.isNone(latest)) {
            return
          }

          const frame = latest.value
          if (frame.ageMs > STALE_FRAME_MS) {
            return
          }

          const prepared = prepareFrame(frame)
          if (prepared.blackout) {
            return
          }

          const reduction = reducePrepared(prepared, yield* Ref.get(reducer), elapsedMs)
          yield* Ref.set(reducer, reduction.state)

          const current = yield* Ref.get(state)
          if (current.inflight || !shouldPublish(reduction.derived, current.last)) {
            return
          }

          const target = reduction.derived
          yield* Ref.set(state, { last: current.last, inflight: true })
          yield* Effect.forkScoped(
            session.setProperties(propertiesFor(target)).pipe(
              Effect.matchEffect({
                onFailure: (error) =>
                  Ref.set(state, {
                    last: error.kind === "device" ? target : current.last,
                    inflight: false,
                  }),
                onSuccess: () => Ref.set(state, { last: target, inflight: false }),
              }),
              Effect.catchCause(() => Ref.set(state, { last: current.last, inflight: false })),
            ),
          )
        })

      const loop = (session: MiioSession): Effect.Effect<void, never, Scope.Scope> =>
        Effect.gen(function* runLoop() {
          const reducer = yield* Ref.make(createReducerState())
          const state = yield* Ref.make<BulbRunState>({ last: null, inflight: false })
          let previousTick = performance.now()
          let deadline = performance.now()

          while (true) {
            const now = performance.now()
            if (now < deadline) {
              yield* Effect.sleep(deadline - now)
            }

            const startedAt = performance.now()
            yield* tick(session, reducer, state, startedAt - previousTick)
            previousTick = startedAt
            deadline += TICK_MS
            while (deadline <= performance.now()) {
              deadline += TICK_MS
            }
          }
        })

      const start = Effect.fn("ScreenSync.start")(function* start() {
        const claimed = yield* Ref.modify(runState, (current): readonly [boolean, RunState] =>
          current.status === "idle" ? [true, { status: "starting" }] : [false, current],
        )
        if (!claimed) {
          return yield* new ScreenSyncError({
            kind: "already-running",
            message: "Screen sync is already running. Press s to stop it.",
          })
        }

        return yield* Effect.gen(function* startRun() {
          const listed = yield* bulbs.list().pipe(Effect.mapError(bulbFailure))
          const bulb = listed[0]
          if (bulb === undefined) {
            return yield* new ScreenSyncError({
              kind: "no-bulbs",
              message: "Add a bulb before starting screen sync.",
            })
          }

          const monitors = yield* capture.listMonitors().pipe(Effect.mapError(captureFailure))
          const monitor = pickMonitor(monitors)
          if (monitor === null) {
            return yield* new ScreenSyncError({
              kind: "no-monitor",
              message: "No monitor to capture. Start jyoti inside a Hyprland session.",
            })
          }

          const ready = yield* Deferred.make<true, ScreenSyncError>()
          const fiber = yield* Effect.gen(function* run() {
            const session = yield* bulbs.openSession(bulb.id).pipe(Effect.mapError(bulbFailure))
            yield* Effect.addFinalizer(() => capture.stop())
            yield* Effect.addFinalizer(() => Ref.set(runState, idleState))
            yield* capture
              .start({ monitor: monitor.name, grid: DEFAULT_GRID })
              .pipe(Effect.mapError(captureFailure))
            yield* Deferred.succeed(ready, true)
            yield* loop(session)
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

          yield* Deferred.await(ready).pipe(Effect.onInterrupt(() => Fiber.interrupt(fiber)))
          const info: SyncRun = { bulbName: bulb.name, monitorName: monitor.name }
          yield* Ref.set(runState, { status: "running", fiber })
          return info
        }).pipe(
          Effect.onExit((exit) =>
            Exit.isFailure(exit) ? Ref.set(runState, idleState) : Effect.void,
          ),
        )
      })

      const stop = Effect.fn("ScreenSync.stop")(function* stop() {
        const current = yield* Ref.getAndSet(runState, idleState)
        if (current.status !== "running") {
          return
        }
        yield* Fiber.interrupt(current.fiber)
        yield* capture.stop()
      })

      const running = Effect.fn("ScreenSync.running")(function* running() {
        return (yield* Ref.get(runState)).status === "running"
      })

      return ScreenSyncEngine.of({ start, stop, running })
    }),
  )
}

export { ScreenSyncEngine, type ScreenSyncShape, type SyncRun }
