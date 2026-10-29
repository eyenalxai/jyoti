import { clamp } from "@/lib/format"

const SYNC_BRIGHTNESS_FLOOR = 8
const SYNC_BRIGHTNESS_CEILING = 100
const SYNC_BRIGHTNESS_SPAN = SYNC_BRIGHTNESS_CEILING - SYNC_BRIGHTNESS_FLOOR

const syncBrightnessFor = (sceneBrightness: number): number =>
  Math.round(
    clamp(
      SYNC_BRIGHTNESS_FLOOR + SYNC_BRIGHTNESS_SPAN * sceneBrightness,
      SYNC_BRIGHTNESS_FLOOR,
      SYNC_BRIGHTNESS_CEILING,
    ),
  )

export { syncBrightnessFor }
