import { createSignal } from "solid-js"

import type { ColorOutcome } from "@/components/bulb/color/outcome"
import type { ColorSnapshot } from "@/components/bulb/color/snapshot"
import type { BulbsModel } from "@/components/bulb/use-bulbs"
import type { ControlsModel } from "@/components/bulb/use-controls"
import type { Notice } from "@/components/ui/toast"
import type { BulbId, BulbState } from "@/lib/bulbs/schema"

interface UseColorSessionOptions {
  readonly model: BulbsModel
  readonly controls: ControlsModel
  readonly notify: (notice: Notice) => void
}

const useColorSession = (options: UseColorSessionOptions) => {
  const [snapshot, setSnapshot] = createSignal<ColorSnapshot>(new Map())
  const [dirty, setDirty] = createSignal(false)
  let send: Promise<unknown> = Promise.resolve()

  const capture = () => {
    const all = options.model.allSelected()
    const selected = options.model.selected()
    const targets = all ? options.model.bulbs() : selected === null ? [] : [selected]
    const states = options.model.states()
    const next = new Map<BulbId, BulbState | null>()
    for (const bulb of targets) {
      next.set(bulb.id, states.get(bulb.id) ?? null)
    }
    setSnapshot(next)
    setDirty(false)
  }

  const commit = (hex: string): Promise<ColorOutcome> => {
    setDirty(true)
    const result = send.then(() => options.controls.setColor(hex))
    send = result.then(
      () => null,
      () => null,
    )
    return result
  }

  const preview = (hex: string): Promise<boolean> =>
    commit(hex).then((outcome) => outcome.error === null)

  const revert = () => {
    const current = snapshot()
    if (current.size === 0) {
      options.notify({ kind: "info", message: "No previous colour was known to revert." })
      return
    }
    void send.then(() => options.controls.revertColors(current))
  }

  const reset = () => {
    setDirty(false)
  }

  return { capture, commit, preview, revert, reset, dirty }
}

export { useColorSession }
