import type { BulbId } from "@/lib/bulbs/schema"
import type { Monitor } from "@/lib/screen/frame"
import type { DerivedColor } from "@/lib/screen/reducer"

const TICK_MS = 667
const STALE_FRAME_MS = 1000
const LOST_FRAME_MS = 5000
const SYNC_BRIGHTNESS_FLOOR = 8
const SYNC_BRIGHTNESS_CEILING = 80
const SYNC_BRIGHTNESS_SPAN = 72

type SyncPhase = "idle" | "starting" | "syncing" | "paused" | "stopping"
type CaptureHealth = "ok" | "switching" | "black" | "stale" | "lost"
type BulbSyncPhase = "syncing" | "unreachable" | "restored" | "not-restored"

interface SyncedBulb {
  readonly id: BulbId
  readonly name: string
  readonly status: BulbSyncPhase
}

interface SyncStatus {
  readonly phase: SyncPhase
  readonly capture: CaptureHealth
  readonly monitor: string | null
  readonly pendingMonitor: string | null
  readonly color: DerivedColor | null
  readonly syncBrightness: number | null
  readonly frameAgeMs: number | null
  readonly reachable: number
  readonly total: number
  readonly selfCapture: boolean
  readonly bulbs: readonly SyncedBulb[]
  readonly error: string | null
}

const idleSyncStatus: SyncStatus = {
  phase: "idle",
  capture: "ok",
  monitor: null,
  pendingMonitor: null,
  color: null,
  syncBrightness: null,
  frameAgeMs: null,
  reachable: 0,
  total: 0,
  selfCapture: false,
  bulbs: [],
  error: null,
}

interface SyncRun {
  readonly monitorName: string
  readonly bulbCount: number
}

const pickDefaultMonitor = (monitors: readonly Monitor[]): Monitor | null => {
  const first = monitors[0]
  if (first === undefined) {
    return null
  }
  if (monitors.length === 1) {
    return first
  }
  let fallback = first
  for (const monitor of monitors) {
    if (monitor.focused === false) {
      fallback = monitor
      break
    }
  }
  return fallback
}

const selfCaptureFor = (monitor: Monitor, sole: boolean): boolean =>
  sole || monitor.focused !== false

const syncBrightnessFor = (sceneL: number): number =>
  Math.round(
    Math.min(
      SYNC_BRIGHTNESS_CEILING,
      Math.max(SYNC_BRIGHTNESS_FLOOR, SYNC_BRIGHTNESS_FLOOR + SYNC_BRIGHTNESS_SPAN * sceneL),
    ),
  )

export {
  LOST_FRAME_MS,
  STALE_FRAME_MS,
  TICK_MS,
  idleSyncStatus,
  pickDefaultMonitor,
  selfCaptureFor,
  syncBrightnessFor,
}
export type { BulbSyncPhase, CaptureHealth, SyncPhase, SyncRun, SyncStatus, SyncedBulb }
