import { clamp } from "@/lib/format"

const SYNC_GAIN_MIN = 0.5
const SYNC_GAIN_MAX = 2
const SYNC_GAIN_PRECISION = 10
const SYNC_GAIN_STEP = 1 / SYNC_GAIN_PRECISION
const SYNC_GAIN_DEFAULT = 1

type SyncGainDirection = -1 | 1

const roundGain = (gain: number): number =>
  Math.round(gain * SYNC_GAIN_PRECISION) / SYNC_GAIN_PRECISION

const normalizeSyncGain = (gain: number): number =>
  clamp(roundGain(gain), SYNC_GAIN_MIN, SYNC_GAIN_MAX)

const nextSyncGain = (gain: number, direction: SyncGainDirection): number =>
  normalizeSyncGain(gain + direction * SYNC_GAIN_STEP)

export { SYNC_GAIN_DEFAULT, nextSyncGain, normalizeSyncGain }
export type { SyncGainDirection }
