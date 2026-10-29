import type { Accessor, Setter } from "solid-js"

import type { Screen } from "@/components/bulb/modals"
import type { BulbsModel } from "@/components/bulb/use-bulbs"
import type { Notice } from "@/components/ui/toast"
import type { Bulb, BulbId, BulbState } from "@/lib/bulbs/schema"

import { SYNC_BLOCKS_BULBS, SYNC_OWNS_CONTROLS } from "@/lib/screen/sync/copy"

interface UseBulbActionsOptions {
  readonly model: BulbsModel
  readonly syncActive: Accessor<boolean>
  readonly busy: Accessor<boolean>
  readonly notify: (notice: Notice) => void
  readonly setActiveBulb: Setter<Bulb | null>
  readonly setColorDirty: Setter<boolean>
  readonly setColorSnapshot: Setter<ReadonlyMap<BulbId, BulbState>>
  readonly setScreen: Setter<Screen>
}

interface BulbActions {
  readonly openAdd: () => void
  readonly openEdit: () => void
  readonly openRemove: () => void
  readonly openColor: () => void
}

const useBulbActions = (options: UseBulbActionsOptions): BulbActions => {
  const bulbChangeBlocked = (): boolean => {
    if (!options.syncActive()) {
      return false
    }
    options.notify({ kind: "info", message: SYNC_BLOCKS_BULBS })
    return true
  }

  const openAdd = () => {
    if (bulbChangeBlocked() || options.busy()) {
      return
    }
    options.setActiveBulb(null)
    options.setScreen("add")
  }

  const openEdit = () => {
    if (bulbChangeBlocked()) {
      return
    }
    if (options.model.allSelected()) {
      options.notify({ kind: "info", message: "All bulbs is a group. Select a bulb to edit it." })
      return
    }
    const bulb = options.model.selected()
    if (bulb === null || options.busy()) {
      return
    }
    options.setActiveBulb(bulb)
    options.setScreen("edit")
  }

  const openRemove = () => {
    if (bulbChangeBlocked()) {
      return
    }
    if (options.model.allSelected()) {
      options.notify({
        kind: "info",
        message: "All bulbs is a group. Select a bulb to remove it.",
      })
      return
    }
    const bulb = options.model.selected()
    if (bulb === null || options.busy()) {
      return
    }
    options.setActiveBulb(bulb)
    options.setScreen("remove")
  }

  const openColor = () => {
    if (options.syncActive()) {
      options.notify({
        kind: "info",
        message: SYNC_OWNS_CONTROLS,
      })
      return
    }
    if (options.busy()) {
      return
    }
    const all = options.model.allSelected()
    const selected = options.model.selected()
    if (!all && selected === null) {
      return
    }

    const targets = all ? options.model.bulbs() : selected === null ? [] : [selected]
    const states = options.model.states()
    const snapshot = new Map<BulbId, BulbState>()
    for (const bulb of targets) {
      const state = states.get(bulb.id)
      if (state !== undefined) {
        snapshot.set(bulb.id, state)
      }
    }

    options.setColorSnapshot(snapshot)
    options.setColorDirty(false)
    options.setActiveBulb(selected)
    options.setScreen("color")
  }

  return { openAdd, openEdit, openRemove, openColor }
}

export { useBulbActions, type BulbActions }
