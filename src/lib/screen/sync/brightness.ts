import { clamp } from "@/lib/format"

const SYNC_BRIGHTNESS_FLOOR = 8
const SYNC_BRIGHTNESS_CEILING = 100
const SYNC_BRIGHTNESS_SPAN = SYNC_BRIGHTNESS_CEILING - SYNC_BRIGHTNESS_FLOOR
const BRIGHTNESS_SLEW = 8

const syncBrightnessFor = (sceneBrightness: number): number =>
  Math.round(
    clamp(
      SYNC_BRIGHTNESS_FLOOR + SYNC_BRIGHTNESS_SPAN * sceneBrightness,
      SYNC_BRIGHTNESS_FLOOR,
      SYNC_BRIGHTNESS_CEILING,
    ),
  )

const slewBrightness = (applied: number | null, target: number): number => {
  if (applied === null) {
    return target
  }
  const step = Math.max(-BRIGHTNESS_SLEW, Math.min(BRIGHTNESS_SLEW, target - applied))
  return applied + step
}

export { slewBrightness, syncBrightnessFor }
