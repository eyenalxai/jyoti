import type { BulbsModel } from "@/components/bulb/use-bulbs"
import type { LightQuality } from "@/lib/bulbs/aggregate"

import { aggregateState, lightQualityOf } from "@/lib/bulbs/aggregate"
import { NEUTRAL_TEMPERATURE } from "@/lib/bulbs/schema"
import { kelvinToSrgb } from "@/lib/color"

const NEUTRAL_COLOR = "#FFFFFF"

const initialColor = (model: BulbsModel): string => {
  const aggregate = model.allSelected()
    ? aggregateState(model.bulbs(), model.states(), model.stateErrors())
    : null
  const quality: LightQuality =
    aggregate === null ? lightQualityOf(model.selectedState()?.mode) : aggregate.quality

  if (quality === "color") {
    const current =
      aggregate === null
        ? model.selectedState()?.color
        : aggregate.color?.kind === "equal"
          ? aggregate.color.hex
          : undefined
    if (current !== undefined && current !== null) {
      return current
    }
  }
  if (quality !== "white") {
    return NEUTRAL_COLOR
  }

  const kelvin =
    aggregate === null
      ? (model.selectedState()?.temperature ?? NEUTRAL_TEMPERATURE)
      : (aggregate.temperature?.low ?? NEUTRAL_TEMPERATURE)
  return kelvinToSrgb(kelvin).hex
}

export { initialColor }
