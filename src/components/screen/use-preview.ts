import type { Accessor } from "solid-js"

import { Effect, Option } from "effect"
import { createSignal, onCleanup } from "solid-js"

import type { ScreenCaptureError } from "@/lib/screen/error"
import type { Monitor } from "@/lib/screen/frame"
import type { DerivedColor } from "@/lib/screen/reducer"
import type { Timer } from "@/lib/timers"

import { forkScreen } from "@/lib/runtime"
import { ScreenCapture } from "@/lib/screen/capture"
import { DEFAULT_GRID } from "@/lib/screen/frame"
import { prepareFrame } from "@/lib/screen/grid"
import { createReducerState, reducePrepared, shouldPublish } from "@/lib/screen/reducer"
import { after, cancel } from "@/lib/timers"

const PREVIEW_TICK_MS = 667

interface PreviewFrame {
  readonly sequence: number
  readonly ageMs: number
  readonly blackout: boolean
  readonly published: boolean
  readonly p90Chroma: number
}

interface CapturePreviewModel {
  readonly monitors: Accessor<readonly Monitor[]>
  readonly selected: Accessor<number>
  readonly running: Accessor<boolean>
  readonly busy: Accessor<boolean>
  readonly loading: Accessor<boolean>
  readonly derived: Accessor<DerivedColor | null>
  readonly frame: Accessor<PreviewFrame | null>
  readonly open: () => void
  readonly move: (delta: number) => void
  readonly toggle: () => void
  readonly close: () => void
}

interface UseCapturePreviewOptions {
  readonly onError: (message: string) => void
}

const useCapturePreview = (options: UseCapturePreviewOptions): CapturePreviewModel => {
  const [monitors, setMonitors] = createSignal<readonly Monitor[]>([])
  const [selected, setSelected] = createSignal(0)
  const [running, setRunning] = createSignal(false)
  const [busy, setBusy] = createSignal(false)
  const [loading, setLoading] = createSignal(false)
  const [derived, setDerived] = createSignal<DerivedColor | null>(null)
  const [frame, setFrame] = createSignal<PreviewFrame | null>(null)
  let reducerState = createReducerState()
  let lastPublished: DerivedColor | null = null
  let lastTick = 0
  let timer: Timer | null = null

  const reportFailure = (failure: ScreenCaptureError): Effect.Effect<void> =>
    Effect.sync(() => {
      options.onError(failure.message)
    })

  const clearTimer = () => {
    if (timer !== null) {
      cancel(timer)
      timer = null
    }
  }

  const stopCapture = () => {
    clearTimer()
    setRunning(false)
    forkScreen(
      Effect.gen(function* stopPreview() {
        const capture = yield* ScreenCapture
        yield* capture.stop()
      }),
    )
  }

  const tick = () => {
    forkScreen(
      Effect.gen(function* readPreview() {
        const capture = yield* ScreenCapture
        const latest = yield* capture.latest()
        if (Option.isNone(latest)) {
          return
        }
        const next = latest.value
        const now = performance.now()
        const elapsed = Math.max(0, now - lastTick)
        lastTick = now
        const prepared = prepareFrame(next)
        if (prepared.blackout) {
          setFrame({
            sequence: next.sequence,
            ageMs: next.ageMs,
            blackout: true,
            published: false,
            p90Chroma: prepared.p90Chroma,
          })
          return
        }
        const reduction = reducePrepared(prepared, reducerState, elapsed)
        reducerState = reduction.state
        const published = shouldPublish(reduction.derived, lastPublished)
        if (published) {
          setDerived(reduction.derived)
          lastPublished = reduction.derived
        }
        setFrame({
          sequence: next.sequence,
          ageMs: next.ageMs,
          blackout: false,
          published,
          p90Chroma: reduction.p90Chroma,
        })
      }),
    )
  }

  const schedule = () => {
    timer = after(PREVIEW_TICK_MS, () => {
      tick()
      if (running()) {
        schedule()
      }
    })
  }

  const startCapture = () => {
    const monitor = monitors()[selected()]
    if (monitor === undefined) {
      options.onError("No monitor to capture.")
      return
    }
    setBusy(true)
    forkScreen(
      Effect.gen(function* startPreview() {
        const capture = yield* ScreenCapture
        yield* capture.start({ monitor: monitor.name, grid: DEFAULT_GRID })
        reducerState = createReducerState()
        lastPublished = null
        lastTick = performance.now()
        setDerived(null)
        setFrame(null)
        setRunning(true)
        schedule()
      }).pipe(
        Effect.catchTag("ScreenCaptureError", reportFailure),
        Effect.ensuring(
          Effect.sync(() => {
            setBusy(false)
          }),
        ),
      ),
    )
  }

  const open = () => {
    setLoading(true)
    forkScreen(
      Effect.gen(function* listPreviewMonitors() {
        const capture = yield* ScreenCapture
        const listed = yield* capture.listMonitors()
        setMonitors(listed)
        setSelected((current) => Math.min(current, Math.max(0, listed.length - 1)))
      }).pipe(
        Effect.catchTag("ScreenCaptureError", reportFailure),
        Effect.ensuring(
          Effect.sync(() => {
            setLoading(false)
          }),
        ),
      ),
    )
  }

  const move = (delta: number) => {
    setSelected((current) => {
      const count = monitors().length
      if (count === 0) {
        return 0
      }
      return (current + delta + count) % count
    })
  }

  const toggle = () => {
    if (busy()) {
      return
    }
    if (running()) {
      stopCapture()
      return
    }
    startCapture()
  }

  const close = () => {
    stopCapture()
    setDerived(null)
    setFrame(null)
  }

  onCleanup(() => {
    clearTimer()
  })

  return {
    monitors,
    selected,
    running,
    busy,
    loading,
    derived,
    frame,
    open,
    move,
    toggle,
    close,
  }
}

export { useCapturePreview, type CapturePreviewModel, type PreviewFrame }
