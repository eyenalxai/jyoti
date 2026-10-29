const SYNC_BRIGHTNESS_FLOOR = 8
const SYNC_BRIGHTNESS_CEILING = 80
const SYNC_BRIGHTNESS_SPAN = 72

const syncBrightnessFor = (sceneL: number): number =>
  Math.round(
    Math.min(
      SYNC_BRIGHTNESS_CEILING,
      Math.max(SYNC_BRIGHTNESS_FLOOR, SYNC_BRIGHTNESS_FLOOR + SYNC_BRIGHTNESS_SPAN * sceneL),
    ),
  )

export { syncBrightnessFor }
