import type { BulbsModel } from "@/components/bulb/use-bulbs"
import type { GroupControlsModel } from "@/components/bulb/use-group-controls"
import type { Bulb, BulbState } from "@/lib/bulbs/schema"

import { Bulbs } from "@/lib/bulbs/service"
import { run } from "@/lib/runtime"

interface UsePowerOptions {
  readonly model: BulbsModel
  readonly onError: (message: string) => void
  readonly group: GroupControlsModel
}

interface PowerModel {
  readonly togglePower: () => Promise<string | null>
}

const usePower = (options: UsePowerOptions): PowerModel => {
  const commitPower = (bulb: Bulb, current: BulbState): Promise<string | null> => {
    const target = current.power === false
    options.model.patchState(bulb.id, { power: target })
    return run(Bulbs.use((service) => service.setPower(bulb.id, target))).then((result) => {
      if (result.ok) {
        return null
      }
      options.onError(result.message)
      void options.model.readOne(bulb.id)
      return result.message
    })
  }

  const togglePower = (): Promise<string | null> => {
    if (options.model.allSelected()) {
      return options.group.togglePower()
    }
    const bulb = options.model.selected()
    if (bulb === null) {
      return Promise.resolve(null)
    }
    const state = options.model.states().get(bulb.id)
    if (state !== undefined && state.power !== null) {
      return commitPower(bulb, state)
    }

    return options.model.readOne(bulb.id).then((message) => {
      if (message !== null) {
        options.onError(message)
        return message
      }
      const fresh = options.model.states().get(bulb.id)
      if (fresh === undefined || fresh.power === null) {
        const unknown = "The power state is unknown. Press r to retry."
        options.onError(unknown)
        return unknown
      }
      return commitPower(bulb, fresh)
    })
  }

  return { togglePower }
}

export { usePower, type PowerModel }
