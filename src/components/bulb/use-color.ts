import type { Accessor } from "solid-js"

import { Array as Arr } from "effect"

import type { BulbsModel } from "@/components/bulb/use-bulbs"
import type { GroupControlsModel } from "@/components/bulb/use-group-controls"
import type { BulbId } from "@/lib/bulbs/schema"
import type { Timer } from "@/lib/timers"

import { normalizeColor } from "@/lib/bulbs/schema"
import { after, cancel } from "@/lib/timers"

interface UseColorOptions {
  readonly model: BulbsModel
  readonly blocked: () => boolean
  readonly syncLocked: Accessor<boolean>
  readonly group: GroupControlsModel
  readonly schedule: (id: BulbId, control: string, send: () => void) => void
  readonly sendColor: (id: BulbId, hex: string) => void
  readonly cancelPending: (id: BulbId, control: string) => void
}

interface ColorModel {
  readonly setColorLive: (hex: string) => void
  readonly flush: () => void
  readonly cancel: () => void
  readonly cancelColorPending: () => void
}

const useColor = (options: UseColorOptions): ColorModel => {
  let timer: Timer | null = null
  let pendingHex: string | null = null

  const cancelTimer = () => {
    if (timer !== null) {
      cancel(timer)
    }
    timer = null
    pendingHex = null
  }

  const sendPending = () => {
    timer = null
    const hex = pendingHex
    pendingHex = null
    if (hex !== null && !options.syncLocked()) {
      void options.group.setColor(hex)
    }
  }

  const flush = () => {
    if (timer === null) {
      return
    }
    cancel(timer)
    sendPending()
  }

  const cancelColorPending = () => {
    for (const bulb of options.model.bulbs()) {
      options.cancelPending(bulb.id, "color")
    }
  }

  const setColorLive = (hex: string) => {
    if (options.blocked()) {
      return
    }
    const color = normalizeColor(hex)
    if (color === null) {
      return
    }
    const states = options.model.states()

    if (options.model.allSelected()) {
      const bulbs = options.model.bulbs()
      const unchanged = Arr.every(bulbs, (bulb) => {
        const state = states.get(bulb.id)
        return state?.color === color && state.mode === "color"
      })
      if (unchanged) {
        return
      }
      for (const bulb of bulbs) {
        options.cancelPending(bulb.id, "temperature")
        options.model.patchState(bulb.id, { color, mode: "color" })
      }
      cancelTimer()
      pendingHex = color
      timer = after(180, sendPending)
      return
    }

    const bulb = options.model.selected()
    if (bulb === null) {
      return
    }
    const state = states.get(bulb.id)
    if (state?.color === color && state.mode === "color") {
      return
    }
    options.cancelPending(bulb.id, "temperature")
    options.model.patchState(bulb.id, { color, mode: "color" })
    options.schedule(bulb.id, "color", () => {
      options.sendColor(bulb.id, color)
    })
  }

  return { setColorLive, flush, cancel: cancelTimer, cancelColorPending }
}

export { useColor, type ColorModel }
