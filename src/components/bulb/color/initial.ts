import type { BulbsModel } from "@/components/bulb/use-bulbs"

import { aggregateState } from "@/lib/bulbs/aggregate"

const initialColor = (model: BulbsModel): string => {
  if (!model.allSelected()) {
    return model.selectedState()?.color ?? "#FFFFFF"
  }
  const aggregate = aggregateState(model.bulbs(), model.states(), model.stateErrors())
  return aggregate.color?.kind === "equal" ? aggregate.color.hex : "#FFFFFF"
}

export { initialColor }
