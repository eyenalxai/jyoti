import type { DerivedColor } from "@/lib/screen/reducer"

import { oklchToSrgb, rgbToOklch } from "@/lib/color"
import { derivedFromSrgb } from "@/lib/screen/reducer"

const gainColor = (derived: DerivedColor, gain: number): DerivedColor => {
  if (gain === 1) {
    return derived
  }
  const source = rgbToOklch(derived.rgb)
  const gained = oklchToSrgb({ ...source, lightness: source.lightness * gain })
  return derivedFromSrgb(gained, derived.sceneBrightness)
}

export { gainColor }
