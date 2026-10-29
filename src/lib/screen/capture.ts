import type { Library } from "bun:ffi"

import { dlopen, FFIType, ptr } from "bun:ffi"
import { Context, Effect, Layer, Option } from "effect"
import path from "node:path"

import type { GridSize, Monitor, ScreenFrame } from "@/lib/screen/frame"

import { ScreenCaptureError } from "@/lib/screen/error"
import { focusedMonitor, readFocusConfig } from "@/lib/screen/focus"
import { DEFAULT_RATE_HZ } from "@/lib/screen/frame"

const MONITOR_NAME_MAX = 64
const MONITOR_DESCRIPTION_MAX = 128
/* Mirrors jyoti_monitor in native/capture.c: two fixed char arrays, then three int32. */
const MONITOR_WIDTH_OFFSET = MONITOR_NAME_MAX + MONITOR_DESCRIPTION_MAX
const MONITOR_HEIGHT_OFFSET = MONITOR_WIDTH_OFFSET + 4
const MONITOR_REFRESH_OFFSET = MONITOR_HEIGHT_OFFSET + 4
const MONITOR_STRUCT_SIZE = MONITOR_REFRESH_OFFSET + 4
const MONITOR_CAPACITY = 16
const FRAME_INFO_SIZE = 16
const FIRST_FRAME_ATTEMPTS = 100
const FIRST_FRAME_POLL_MS = 20

const SYMBOLS = {
  jyoti_capture_list_monitors: { args: [FFIType.ptr, FFIType.i32], returns: FFIType.i32 },
  jyoti_capture_start: {
    args: [FFIType.cstring, FFIType.i32, FFIType.i32, FFIType.i32],
    returns: FFIType.i32,
  },
  jyoti_capture_stop: { args: [], returns: FFIType.void },
  jyoti_capture_latest: {
    args: [FFIType.ptr, FFIType.u64, FFIType.ptr],
    returns: FFIType.u64,
  },
  jyoti_capture_error: { args: [], returns: FFIType.cstring },
} as const

type CaptureLibrary = Library<typeof SYMBOLS>

const LIBRARY_PATH = path.resolve(import.meta.dir, "../../../native/build/libjyoti-capture.so")

interface StartOptions {
  readonly monitor: string
  readonly grid: GridSize
  readonly rateHz?: number
}

interface ScreenCaptureShape {
  readonly listMonitors: () => Effect.Effect<readonly Monitor[], ScreenCaptureError>
  readonly start: (options: StartOptions) => Effect.Effect<void, ScreenCaptureError>
  readonly latest: () => Effect.Effect<Option.Option<ScreenFrame>>
  readonly stop: () => Effect.Effect<void>
}

let library: CaptureLibrary | null = null
let started = false
let activeGrid: GridSize | null = null

const decodeText = (buffer: Uint8Array, offset: number, length: number): string => {
  const bytes = buffer.subarray(offset, offset + length)
  const end = bytes.indexOf(0)
  return new TextDecoder().decode(end === -1 ? bytes : bytes.subarray(0, end))
}

const decodeMonitor = (buffer: Uint8Array, index: number): Monitor => {
  const view = new DataView(buffer.buffer, buffer.byteOffset, buffer.byteLength)
  const base = index * MONITOR_STRUCT_SIZE
  return {
    name: decodeText(buffer, base, MONITOR_NAME_MAX),
    description: decodeText(buffer, base + MONITOR_NAME_MAX, MONITOR_DESCRIPTION_MAX),
    width: view.getInt32(base + MONITOR_WIDTH_OFFSET, true),
    height: view.getInt32(base + MONITOR_HEIGHT_OFFSET, true),
    refreshHz: view.getInt32(base + MONITOR_REFRESH_OFFSET, true) / 1000,
    focused: null,
  }
}

const errorKind = (code: number): ScreenCaptureError["kind"] => {
  if (code === -2) {
    return "monitor"
  }
  if (code === -3) {
    return "wayland"
  }
  return "capture"
}

const loadLibrary = (): Effect.Effect<CaptureLibrary, ScreenCaptureError> => {
  if (library !== null) {
    return Effect.succeed(library)
  }
  try {
    const opened = dlopen(LIBRARY_PATH, SYMBOLS)
    library = opened
    return Effect.succeed(opened)
  } catch (error) {
    return Effect.fail(
      new ScreenCaptureError({
        kind: "library",
        message: "The screen capture library is missing. Run bun run build:native, then retry.",
        cause: error,
      }),
    )
  }
}

const readLatest = (opened: CaptureLibrary, grid: GridSize): Option.Option<ScreenFrame> => {
  const pixels = new Uint8Array(grid.width * grid.height * 4)
  const info = new Uint8Array(FRAME_INFO_SIZE)
  const sequence = opened.symbols.jyoti_capture_latest(ptr(pixels), pixels.byteLength, ptr(info))
  if (sequence === 0n) {
    return Option.none()
  }
  const view = new DataView(info.buffer)
  return Option.some({
    pixels,
    width: view.getInt32(0, true),
    height: view.getInt32(4, true),
    sequence: Number(sequence),
    ageMs: Number(view.getBigInt64(8, true)),
  })
}

const stopNow = (): void => {
  if (!started) {
    return
  }
  library?.symbols.jyoti_capture_stop()
  started = false
  activeGrid = null
}

const closeLibrary = (): void => {
  if (library === null) {
    return
  }
  library.close()
  library = null
}

class ScreenCapture extends Context.Service<ScreenCapture, ScreenCaptureShape>()(
  "jyoti/lib/screen/capture/ScreenCapture",
) {
  static readonly layer = Layer.effect(
    ScreenCapture,
    Effect.gen(function* layer() {
      const focusConfig = yield* readFocusConfig()

      yield* Effect.addFinalizer(() =>
        Effect.sync(() => {
          stopNow()
          closeLibrary()
        }),
      )

      const listMonitors = Effect.fn("ScreenCapture.listMonitors")(function* listMonitors() {
        const opened = yield* loadLibrary()
        const buffer = new Uint8Array(MONITOR_STRUCT_SIZE * MONITOR_CAPACITY)
        const count = opened.symbols.jyoti_capture_list_monitors(ptr(buffer), MONITOR_CAPACITY)
        if (count < 0) {
          return yield* new ScreenCaptureError({
            kind: "wayland",
            message:
              opened.symbols.jyoti_capture_error() ??
              "Could not list monitors. Start jyoti inside a Hyprland session.",
          })
        }
        const focused = yield* focusedMonitor(focusConfig.signature, focusConfig.runtimeDir)
        const monitors: Monitor[] = []
        for (let index = 0; index < count && index < MONITOR_CAPACITY; index += 1) {
          const monitor = decodeMonitor(buffer, index)
          monitors.push({
            ...monitor,
            focused: focused === null ? null : monitor.name === focused,
          })
        }
        return monitors
      })

      const waitForFirstFrame = Effect.fn("ScreenCapture.waitForFirstFrame")(
        function* waitForFirstFrame(opened: CaptureLibrary, grid: GridSize) {
          for (let attempt = 0; attempt < FIRST_FRAME_ATTEMPTS; attempt += 1) {
            if (Option.isSome(readLatest(opened, grid))) {
              return true
            }
            yield* Effect.sleep(FIRST_FRAME_POLL_MS)
          }
          return false
        },
      )

      const start = Effect.fn("ScreenCapture.start")(function* start(options: StartOptions) {
        const opened = yield* loadLibrary()
        stopNow()
        const rateHz = options.rateHz ?? DEFAULT_RATE_HZ
        const result = opened.symbols.jyoti_capture_start(
          options.monitor,
          options.grid.width,
          options.grid.height,
          rateHz,
        )
        if (result < 0) {
          return yield* new ScreenCaptureError({
            kind: errorKind(result),
            message: opened.symbols.jyoti_capture_error() ?? "Capture did not start.",
          })
        }
        started = true
        activeGrid = options.grid
        const frame = yield* waitForFirstFrame(opened, options.grid)
        if (!frame) {
          stopNow()
          return yield* new ScreenCaptureError({
            kind: "capture",
            message: `Capture started but "${options.monitor}" produced no frame within 2 s.`,
          })
        }
        return yield* Effect.void
      })

      const latest = Effect.fn("ScreenCapture.latest")(() =>
        Effect.sync(() => {
          if (!started || library === null || activeGrid === null) {
            return Option.none<ScreenFrame>()
          }
          return readLatest(library, activeGrid)
        }),
      )

      const stop = Effect.fn("ScreenCapture.stop")(function* stop() {
        yield* Effect.sync(() => {
          stopNow()
        })
      })

      return ScreenCapture.of({ listMonitors, start, latest, stop })
    }),
  )
}

export { ScreenCapture, type ScreenCaptureShape, type StartOptions }
