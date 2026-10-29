import type { Accessor, Setter } from "solid-js"

import { createEffect, createSignal } from "solid-js"

import type { Screen } from "@/components/bulb/modals"
import type { SyncRow } from "@/components/screen/rows"
import type { ScreenSyncModel } from "@/components/screen/use-sync"
import type { SyncGainDirection } from "@/lib/screen/sync/gain"

import { syncRowsFor } from "@/components/screen/rows"

interface UseSyncPanelOptions {
  readonly sync: ScreenSyncModel
  readonly setScreen: Setter<Screen>
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
  const running = () => {
    const phase = options.sync.status().phase
    return phase === "syncing" || phase === "paused"
  }
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
      if (phase === "idle" || phase === "syncing" || phase === "paused") {
        options.sync.toggle()
      }
      return
    }
    if (syncRow() === "pause" && (phase === "syncing" || phase === "paused")) {
      options.sync.togglePause()
      return
    }
    if (syncRow() === "monitor" && phase !== "stopping") {
      options.sync.openMonitors()
      options.setScreen("monitors")
    }
  }

  const adjust = (direction: SyncGainDirection) => {
    if (syncRow() === "gain" && running()) {
      options.sync.adjustGain(direction)
    }
  }

  return { syncRow, open, close, move, activate, adjust }
}

export { useSyncPanel, type SyncPanelModel }
