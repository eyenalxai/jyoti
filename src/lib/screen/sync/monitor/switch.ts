import { Effect, Ref, Result } from "effect"

import type { Monitor } from "@/lib/screen/frame"
import type { RunDeps, RunHandle } from "@/lib/screen/sync/run"

import { ScreenSyncError } from "@/lib/screen/error"
import { DEFAULT_GRID } from "@/lib/screen/frame"
import { createReducerState } from "@/lib/screen/reducer"
import { captureFailure } from "@/lib/screen/sync/failure"
import { selfCaptureFor } from "@/lib/screen/sync/monitor"
import { resetAcked } from "@/lib/screen/sync/progress"
import { patchStatus, publishLive } from "@/lib/screen/sync/run"

const switchMonitor = Effect.fn("ScreenSync.switchMonitor")(function* switchMonitor(
  deps: RunDeps,
  handle: RunHandle,
  monitor: Monitor,
  sole: boolean,
) {
  if (yield* Ref.get(handle.switching)) {
    return yield* new ScreenSyncError({
      kind: "not-ready",
      message: "A monitor switch is already in progress.",
    })
  }

  yield* Ref.set(handle.switching, true)
  yield* patchStatus(deps, { capture: "switching" })

  const switched = yield* Effect.gen(function* switchCapture() {
    yield* deps.capture.stop()
    yield* deps.capture
      .start({ monitor: monitor.name, grid: DEFAULT_GRID })
      .pipe(Effect.mapError(captureFailure))
    yield* Ref.set(handle.monitor, monitor)
    yield* Ref.set(handle.reducer, createReducerState())
    yield* Ref.update(handle.progress, resetAcked)
    yield* publishLive(deps, handle, {
      monitor: monitor.name,
      capture: "ok",
      frameAgeMs: null,
      selfCapture: selfCaptureFor(monitor, sole),
      error: null,
    })
  }).pipe(Effect.result, Effect.ensuring(Ref.set(handle.switching, false)))

  if (Result.isFailure(switched)) {
    const message = switched.failure.message
    yield* patchStatus(deps, { capture: "lost", error: message })
    return yield* new ScreenSyncError({ kind: "capture", message })
  }
  return yield* Effect.void
})

export { switchMonitor }
