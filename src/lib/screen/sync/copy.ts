import { Array as Arr } from "effect"

import type { RestoreReport } from "@/lib/screen/sync/restore"

const SYNC_OWNS_CONTROLS = "sync owns colour, brightness and temperature"
const SYNC_BLOCKS_BULBS = "stop sync to change bulbs"
const SYNC_CANCELLED_MESSAGE = "Screen sync start cancelled."

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
}
