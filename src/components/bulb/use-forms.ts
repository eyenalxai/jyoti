import type { Setter } from "solid-js"

import { String as Str } from "effect"

import type { Screen } from "@/components/bulb/modals"
import type { BulbsModel } from "@/components/bulb/use-bulbs"
import type { ControlsModel } from "@/components/bulb/use-controls"
import type { Notice } from "@/components/ui/status"
import type { Bulb, BulbDraft, BulbPatch } from "@/lib/bulbs/schema"

interface UseFormsOptions {
  readonly model: BulbsModel
  readonly controls: ControlsModel
  readonly activeBulb: () => Bulb | null
  readonly busy: () => boolean
  readonly setActiveBulb: Setter<Bulb | null>
  readonly setBusy: Setter<boolean>
  readonly setFormError: Setter<string | null>
  readonly setScreen: Setter<Screen>
  readonly notify: (notice: Notice) => void
}

interface FormsModel {
  readonly submitAdd: (draft: BulbDraft) => void
  readonly submitEdit: (draft: BulbDraft) => void
  readonly confirmRemove: () => void
  readonly submitColor: () => void
}

const useForms = (options: UseFormsOptions): FormsModel => {
  const submitAdd = (draft: BulbDraft) => {
    options.setBusy(true)
    options.setFormError(null)
    void options.model.add(draft).then((error) => {
      options.setBusy(false)
      if (error !== null) {
        options.setFormError(error)
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
    options.setFormError(null)
    void options.model.update(bulb.id, patch).then((error) => {
      options.setBusy(false)
      if (error !== null) {
        options.setFormError(error)
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
    options.setFormError(null)
    void options.model.remove(bulb.id).then((error) => {
      options.setBusy(false)
      if (error !== null) {
        options.setFormError(error)
        return null
      }
      options.setScreen("dashboard")
      options.setActiveBulb(null)
      options.notify({ kind: "success", message: `${bulb.name} removed.` })
      return null
    })
  }

  const submitColor = () => {
    options.controls.flushPending()
    options.setFormError(null)
    options.setScreen("dashboard")
    options.setActiveBulb(null)
  }

  return { submitAdd, submitEdit, confirmRemove, submitColor }
}

export { useForms, type FormsModel }
