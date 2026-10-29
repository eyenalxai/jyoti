import type { Accessor, Setter } from "solid-js"

import { createEffect, createSignal } from "solid-js"

import type { Screen } from "@/components/bulb/modals"
import type { SyncRow } from "@/components/screen/sync-rows"
import type { ScreenSyncModel } from "@/components/screen/use-sync"
import type { SyncGainDirection } from "@/lib/screen/sync/gain"

import { syncRowsFor } from "@/components/screen/sync-rows"
import { isSyncRunning } from "@/lib/screen/sync/status"

interface UseSyncPanelOptions {
  readonly sync: ScreenSyncModel
  readonly setScreen: Setter<Screen>
  readonly onMonitors: () => void
}

interface SyncPanelModel {
  readonly syncRow: Accessor<SyncRow>
  readonly open: () => void
  readonly close: () => void
  readonly move: (delta: number) => void
  readonly activate: () => void
  readonly adjust: (direction: SyncGainDirection) => void
}

const useSyncPanel = (options: UseSyncPanelOptions): SyncPanelModel => {
  const [syncRow, setSyncRow] = createSignal<SyncRow>("action")
  const running = () => isSyncRunning(options.sync.status().phase)
  const rows = () => syncRowsFor(running())

  createEffect(() => {
    if (!rows().includes(syncRow())) {
      setSyncRow("action")
    }
  })

  const open = () => {
    setSyncRow("action")
    options.setScreen("sync")
  }

  const close = () => {
    options.setScreen("dashboard")
  }

  const move = (delta: number) => {
    const list = rows()
    const index = list.indexOf(syncRow())
    const next = list[(index + delta + list.length) % list.length]
    if (next !== undefined) {
      setSyncRow(next)
    }
  }

  const activate = () => {
    const phase = options.sync.status().phase
    if (syncRow() === "action") {
      if (phase === "idle" || phase === "starting" || phase === "syncing" || phase === "paused") {
        options.sync.toggle()
      }
      return
    }
    if (syncRow() === "pause" && (phase === "syncing" || phase === "paused")) {
      options.sync.togglePause()
      return
    }
    if (syncRow() === "monitor" && phase !== "stopping") {
      options.onMonitors()
    }
  }

  const adjust = (direction: SyncGainDirection) => {
    if (syncRow() === "gain") {
      options.sync.adjustGain(direction)
    }
  }

  return { syncRow, open, close, move, activate, adjust }
}

export { useSyncPanel, type SyncPanelModel }
