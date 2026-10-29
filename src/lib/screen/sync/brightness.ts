import { clamp } from "@/lib/format"

const SYNC_BRIGHTNESS_FLOOR = 8
const SYNC_BRIGHTNESS_CEILING = 100
const SYNC_BRIGHTNESS_SPAN = SYNC_BRIGHTNESS_CEILING - SYNC_BRIGHTNESS_FLOOR

const SYNC_GAIN_MIN = 0.5
const SYNC_GAIN_MAX = 2
const SYNC_GAIN_PRECISION = 10
const SYNC_GAIN_STEP = 1 / SYNC_GAIN_PRECISION
const SYNC_GAIN_DEFAULT = 1

type SyncGainDirection = -1 | 1

const roundGain = (gain: number): number =>
  Math.round(gain * SYNC_GAIN_PRECISION) / SYNC_GAIN_PRECISION

const syncBrightnessFor = (sceneBrightness: number, gain: number): number => {
  const envelope = Math.round(
    clamp(
      SYNC_BRIGHTNESS_FLOOR + SYNC_BRIGHTNESS_SPAN * sceneBrightness,
      SYNC_BRIGHTNESS_FLOOR,
      SYNC_BRIGHTNESS_CEILING,
    ),
  )
  return Math.round(clamp(envelope * gain, SYNC_BRIGHTNESS_FLOOR, SYNC_BRIGHTNESS_CEILING))
}

const nextSyncGain = (gain: number, direction: SyncGainDirection): number =>
  clamp(roundGain(gain + direction * SYNC_GAIN_STEP), SYNC_GAIN_MIN, SYNC_GAIN_MAX)

export { SYNC_GAIN_DEFAULT, nextSyncGain, syncBrightnessFor }
export type { SyncGainDirection }
