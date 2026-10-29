const FLASH_WINDOW = 3
const FLASH_HOLD_MS = 2000
const FLASH_PEAK_TO_PEAK = 25

interface FlashGuard {
  readonly targets: readonly number[]
  readonly heldUntil: number | null
  readonly armed: boolean
}

interface FlashVerdict {
  readonly guard: FlashGuard
  readonly flashing: boolean
}

const initialFlashGuard = (): FlashGuard => ({ targets: [], heldUntil: null, armed: true })

const deltas = (targets: readonly number[]): readonly [number, number] | null => {
  if (targets.length < FLASH_WINDOW) {
    return null
  }
  return [
    Math.sign((targets[1] ?? 0) - (targets[0] ?? 0)),
    Math.sign((targets[2] ?? 0) - (targets[1] ?? 0)),
  ]
}

const sameDirection = (targets: readonly number[]): boolean => {
  const pair = deltas(targets)
  return pair !== null && pair[0] !== 0 && pair[0] === pair[1]
}

const flipsDirection = (targets: readonly number[]): boolean => {
  const pair = deltas(targets)
  return pair !== null && pair[0] !== 0 && pair[1] !== 0 && pair[0] !== pair[1]
}

const peakToPeak = (targets: readonly number[]): number =>
  Math.max(...targets) - Math.min(...targets)

const isFlashing = (guard: FlashGuard, now: number): boolean =>
  guard.heldUntil !== null && now < guard.heldUntil

const guardFlash = (guard: FlashGuard, target: number, now: number): FlashVerdict => {
  const targets = [...guard.targets, target].slice(-FLASH_WINDOW)
  if (isFlashing(guard, now)) {
    return { guard: { ...guard, targets }, flashing: true }
  }

  let armed = guard.heldUntil === null ? guard.armed : false
  if (!armed && (sameDirection(targets) || peakToPeak(targets) <= FLASH_PEAK_TO_PEAK)) {
    armed = true
  }
  if (armed && flipsDirection(targets) && peakToPeak(targets) > FLASH_PEAK_TO_PEAK) {
    return {
      guard: { targets, heldUntil: now + FLASH_HOLD_MS, armed: false },
      flashing: true,
    }
  }
  return { guard: { targets, heldUntil: null, armed }, flashing: false }
}

export { guardFlash, initialFlashGuard, isFlashing }
export type { FlashGuard }
