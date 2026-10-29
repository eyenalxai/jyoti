const SYNC_BRIGHTNESS_FLOOR = 8
const SYNC_BRIGHTNESS_CEILING = 80
const SYNC_BRIGHTNESS_SPAN = 72
const BRIGHTNESS_SLEW = 8

const syncBrightnessFor = (sceneL: number): number =>
  Math.round(
    Math.min(
      SYNC_BRIGHTNESS_CEILING,
      Math.max(SYNC_BRIGHTNESS_FLOOR, SYNC_BRIGHTNESS_FLOOR + SYNC_BRIGHTNESS_SPAN * sceneL),
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
