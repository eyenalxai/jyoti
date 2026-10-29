import type { SyncPhase } from "@/lib/screen/sync/status"

const SYNC_OWNS_CONTROLS = "sync owns colour, brightness and temperature"
const SYNC_BLOCKS_BULBS = "stop sync to change bulbs"
const SYNC_CANCELLED_MESSAGE = "Screen sync start cancelled."

const syncKeys = (phase: SyncPhase): string => {
  if (phase === "starting") {
    return "s cancel · m monitor"
  }
  if (phase === "paused") {
    return "s stop · x resume · m monitor"
  }
  if (phase === "stopping") {
    return "stopping…"
  }
  if (phase === "syncing") {
    return "s stop · x pause · m monitor"
  }
  return "s sync"
}

export { SYNC_BLOCKS_BULBS, SYNC_CANCELLED_MESSAGE, SYNC_OWNS_CONTROLS, syncKeys }
