import type { Setter } from "solid-js"

import { String as Str } from "effect"

import type { Screen } from "@/components/bulb/modals"
import type { BulbsModel } from "@/components/bulb/use-bulbs"
import type { ControlsModel } from "@/components/bulb/use-controls"
import type { Notice } from "@/components/ui/toast"
import type { Bulb, BulbDraft, BulbId, BulbPatch, BulbState } from "@/lib/bulbs/schema"

interface UseFormsOptions {
  readonly model: BulbsModel
  readonly controls: ControlsModel
  readonly activeBulb: () => Bulb | null
  readonly busy: () => boolean
  readonly colorSnapshot: () => ReadonlyMap<BulbId, BulbState>
  readonly setActiveBulb: Setter<Bulb | null>
  readonly setBusy: Setter<boolean>
  readonly setScreen: Setter<Screen>
  readonly notify: (notice: Notice) => void
}

interface FormsModel {
  readonly submitAdd: (draft: BulbDraft) => void
  readonly submitEdit: (draft: BulbDraft) => void
  readonly confirmRemove: () => void
  readonly submitColor: (hex: string) => void
  readonly revertColor: () => void
}

const useForms = (options: UseFormsOptions): FormsModel => {
  const submitAdd = (draft: BulbDraft) => {
    options.setBusy(true)
    void options.model.add(draft).then((error) => {
      options.setBusy(false)
      if (error !== null) {
        options.notify({ kind: "error", message: error })
        return null
      }
      options.setScreen("dashboard")
      options.setActiveBulb(null)
      options.notify({ kind: "success", message: `${Str.trim(draft.name)} added.` })
      return null
    })
  }

  const submitEdit = (draft: BulbDraft) => {
    const bulb = options.activeBulb()
    if (bulb === null) {
      return
    }
    const patch: BulbPatch = {
      name: draft.name,
      address: draft.address,
      ...(draft.model === undefined ? {} : { model: draft.model }),
      ...(draft.token.length === 0 ? {} : { token: draft.token }),
    }
    options.setBusy(true)
    void options.model.update(bulb.id, patch).then((error) => {
      options.setBusy(false)
      if (error !== null) {
        options.notify({ kind: "error", message: error })
        return null
      }
      options.setScreen("dashboard")
      options.setActiveBulb(null)
      options.notify({ kind: "success", message: `${Str.trim(draft.name)} saved.` })
      return null
    })
  }

  const confirmRemove = () => {
    const bulb = options.activeBulb()
    if (bulb === null || options.busy()) {
      return
    }
    options.setBusy(true)
    void options.model.remove(bulb.id).then((error) => {
      options.setBusy(false)
      if (error !== null) {
        options.notify({ kind: "error", message: error })
        return null
      }
      options.setScreen("dashboard")
      options.setActiveBulb(null)
      options.notify({ kind: "success", message: `${bulb.name} removed.` })
      return null
    })
  }

  const submitColor = (hex: string) => {
    const all = options.model.allSelected()
    const bulb = options.activeBulb()
    if ((!all && bulb === null) || options.busy()) {
      return
    }
    const label = all ? "All bulbs" : (bulb?.name ?? "The bulb")
    options.setBusy(true)
    void options.controls.setColor(hex).then((outcome) => {
      options.setBusy(false)
      if (outcome.error !== null && !outcome.partial) {
        return null
      }
      options.setScreen("dashboard")
      options.setActiveBulb(null)
      if (outcome.error === null) {
        options.notify({ kind: "success", message: `${label} set to ${hex}.` })
      }
      return null
    })
  }

  const revertColor = () => {
    const snapshot = options.colorSnapshot()
    if (snapshot.size === 0) {
      options.notify({ kind: "info", message: "No previous colour was known to restore." })
      return
    }
    void options.controls.revertColors(snapshot)
  }

  return { submitAdd, submitEdit, confirmRemove, submitColor, revertColor }
}

export { useForms, type FormsModel }
