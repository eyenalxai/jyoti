import type { BulbId } from "@/lib/bulbs/schema"
import type { DerivedColor } from "@/lib/screen/reducer"

import { SYNC_GAIN_DEFAULT } from "@/lib/screen/sync/brightness"

type SyncPhase = "idle" | "starting" | "syncing" | "paused" | "stopping"
type CaptureHealth = "ok" | "switching" | "stale" | "lost"
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
  readonly brightnessGain: number
  readonly flashing: boolean
  readonly frameAgeMs: number | null
  readonly reachable: number
  readonly total: number
  readonly selfCapture: boolean
  readonly bulbs: readonly SyncedBulb[]
  readonly error: string | null
  readonly rejection: string | null
}

const idleSyncStatus: SyncStatus = {
  phase: "idle",
  capture: "ok",
  monitor: null,
  pendingMonitor: null,
  color: null,
  syncBrightness: null,
  brightnessGain: SYNC_GAIN_DEFAULT,
  flashing: false,
  frameAgeMs: null,
  reachable: 0,
  total: 0,
  selfCapture: false,
  bulbs: [],
  error: null,
  rejection: null,
}

interface SyncRun {
  readonly monitorName: string
  readonly bulbCount: number
}

export { idleSyncStatus }
export type { BulbSyncPhase, CaptureHealth, SyncPhase, SyncRun, SyncStatus, SyncedBulb }
