type SyncRow = "action" | "pause" | "monitor" | "gain"

const syncRowsFor = (running: boolean): readonly SyncRow[] =>
  running ? ["action", "pause", "monitor", "gain"] : ["action", "monitor", "gain"]

export { syncRowsFor }
export type { SyncRow }
