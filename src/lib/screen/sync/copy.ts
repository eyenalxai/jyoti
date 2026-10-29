import { Array as Arr } from "effect"

import type { RestoreReport } from "@/lib/screen/sync/restore"
import type { SyncPhase } from "@/lib/screen/sync/status"

const SYNC_OWNS_CONTROLS = "sync owns colour, brightness and temperature"
const SYNC_BLOCKS_BULBS = "stop sync to change bulbs"
const SYNC_CANCELLED_MESSAGE = "Screen sync start cancelled."

const syncKeys = (phase: SyncPhase): string => {
  if (phase === "starting") {
    return "s cancel · m monitor"
  }
  if (phase === "paused") {
    return "s stop · x resume · m monitor · +/- gain"
  }
  if (phase === "stopping") {
    return "stopping…"
  }
  if (phase === "syncing") {
    return "s stop · x pause · m monitor · +/- gain"
  }
  return "s sync"
}

const restoreSummary = (restored: number, total: number, names: readonly string[]): string => {
  const count = `${restored}/${total} restored`
  return names.length === 0 ? count : `${count} · not restored: ${Arr.join(names, ", ")}`
}

const stoppedMessage = (report: RestoreReport): string => {
  if (report.total === 0) {
    return "Screen sync stopped."
  }
  return `Screen sync stopped · ${restoreSummary(report.restored, report.total, report.notRestored)}`
}

export {
  SYNC_BLOCKS_BULBS,
  SYNC_CANCELLED_MESSAGE,
  SYNC_OWNS_CONTROLS,
  restoreSummary,
  stoppedMessage,
  syncKeys,
}
