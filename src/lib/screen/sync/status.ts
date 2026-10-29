import type { BulbId } from "@/lib/bulbs/schema"
import type { DerivedColor } from "@/lib/screen/reducer"

type SyncPhase = "idle" | "starting" | "syncing" | "paused" | "stopping"
type CaptureHealth = "ok" | "switching" | "black" | "stale" | "lost"
// Restore results arrive with JYO-30; the rail and status already carry their states.
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

export { idleSyncStatus }
export type { BulbSyncPhase, CaptureHealth, SyncPhase, SyncRun, SyncStatus, SyncedBulb }
