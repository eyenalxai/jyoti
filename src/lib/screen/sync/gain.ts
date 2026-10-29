import { clamp } from "@/lib/format"

const SYNC_GAIN_MIN = 0.5
const SYNC_GAIN_MAX = 2
const SYNC_GAIN_PRECISION = 10
const SYNC_GAIN_STEP = 1 / SYNC_GAIN_PRECISION
const SYNC_GAIN_DEFAULT = 1

type SyncGainDirection = -1 | 1

const roundGain = (gain: number): number =>
  Math.round(gain * SYNC_GAIN_PRECISION) / SYNC_GAIN_PRECISION

const nextSyncGain = (gain: number, direction: SyncGainDirection): number =>
  clamp(roundGain(gain + direction * SYNC_GAIN_STEP), SYNC_GAIN_MIN, SYNC_GAIN_MAX)

export { SYNC_GAIN_DEFAULT, nextSyncGain }
export type { SyncGainDirection }
