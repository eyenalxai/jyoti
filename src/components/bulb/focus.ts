import type { ControlFocus } from "@/components/bulb/controls"
import type { LightQuality } from "@/lib/bulbs/aggregate"

const focusOrder = (quality: LightQuality): readonly ControlFocus[] => {
  if (quality === "white") {
    return ["power", "quality", "brightness", "light"]
  }
  if (quality === "color") {
    return ["power", "quality", "light"]
  }
  return ["power", "quality"]
}

const canFocus = (quality: LightQuality, focus: ControlFocus): boolean =>
  focusOrder(quality).includes(focus)

const refusalFor = (focus: ControlFocus): string =>
  focus === "brightness" ? "Choose White to set brightness." : "Choose White or Colour first."

export { canFocus, focusOrder, refusalFor }
