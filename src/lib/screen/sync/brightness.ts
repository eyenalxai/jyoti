const SYNC_BRIGHTNESS_FLOOR = 8
const SYNC_BRIGHTNESS_CEILING = 100
const SYNC_BRIGHTNESS_SPAN = 92
const BRIGHTNESS_SLEW = 8

const syncBrightnessFor = (sceneDrive: number): number =>
  Math.round(
    Math.min(
      SYNC_BRIGHTNESS_CEILING,
      Math.max(SYNC_BRIGHTNESS_FLOOR, SYNC_BRIGHTNESS_FLOOR + SYNC_BRIGHTNESS_SPAN * sceneDrive),
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
