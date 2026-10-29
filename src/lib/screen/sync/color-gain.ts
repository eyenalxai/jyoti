import type { DerivedColor } from "@/lib/screen/reducer"

import { oklchToSrgb } from "@/lib/color"
import { clamp } from "@/lib/format"
import { oklchFromLab } from "@/lib/screen/lab"
import { derivedFromSrgb } from "@/lib/screen/reducer"

const gainColor = (derived: DerivedColor, gain: number): DerivedColor => {
  if (gain === 1) {
    return derived
  }
  const gained = oklchToSrgb(oklchFromLab({ ...derived.lab, l: clamp(derived.lab.l * gain, 0, 1) }))
  return derivedFromSrgb(gained, derived.sceneBrightness)
}

export { gainColor }
