import type { Bulb, BulbId } from "@/lib/bulbs/schema"
import type { MiioError } from "@/lib/miio/error"
import type { PropertyParam } from "@/lib/miio/properties"
import type { MiioSession } from "@/lib/miio/session"
import type { DerivedColor } from "@/lib/screen/reducer"
import type { BulbSyncPhase } from "@/lib/screen/sync/status"

import { PIID_COLOR, PIID_TEMPERATURE, packColor, parseColor } from "@/lib/bulbs/schema"
import { propertiesWithValue } from "@/lib/miio/properties"
import { NEUTRAL_KELVIN } from "@/lib/screen/reducer"

const UNREACHABLE_MISSES = 3

interface BulbProgress {
  readonly last: DerivedColor | null
  readonly inflight: boolean
  readonly misses: number
  readonly status: BulbSyncPhase
}

interface BulbSession {
  readonly bulb: Bulb
  readonly session: MiioSession
}

const propertiesFor = (target: DerivedColor): readonly PropertyParam[] => {
  if (target.mode === "temperature") {
    return propertiesWithValue(PIID_TEMPERATURE, target.kelvin ?? NEUTRAL_KELVIN)
  }
  const rgb = parseColor(target.hex)
  if (rgb === null) {
    return propertiesWithValue(PIID_TEMPERATURE, NEUTRAL_KELVIN)
  }
  return propertiesWithValue(PIID_COLOR, packColor(rgb))
}

const initialProgress = (
  bulbs: readonly Bulb[],
  reachable: ReadonlySet<BulbId>,
): Map<BulbId, BulbProgress> => {
  const progress = new Map<BulbId, BulbProgress>()
  for (const bulb of bulbs) {
    progress.set(bulb.id, {
      last: null,
      inflight: false,
      misses: 0,
      status: reachable.has(bulb.id) ? "syncing" : "unreachable",
    })
  }
  return progress
}

const markInflight = (
  progress: ReadonlyMap<BulbId, BulbProgress>,
  id: BulbId,
): ReadonlyMap<BulbId, BulbProgress> => {
  const current = progress.get(id)
  if (current === undefined) {
    return progress
  }
  return new Map(progress).set(id, { ...current, inflight: true })
}

const settleProgress = (
  progress: ReadonlyMap<BulbId, BulbProgress>,
  id: BulbId,
  target: DerivedColor,
  error: MiioError | null,
): ReadonlyMap<BulbId, BulbProgress> => {
  const current = progress.get(id)
  if (current === undefined) {
    return progress
  }
  if (error === null || error.kind === "device") {
    return new Map(progress).set(id, {
      ...current,
      inflight: false,
      last: target,
      misses: 0,
      status: "syncing",
    })
  }
  const misses = current.misses + 1
  return new Map(progress).set(id, {
    ...current,
    inflight: false,
    misses,
    status: misses >= UNREACHABLE_MISSES ? "unreachable" : current.status,
  })
}

const missProgress = (
  progress: ReadonlyMap<BulbId, BulbProgress>,
  id: BulbId,
): ReadonlyMap<BulbId, BulbProgress> => {
  const current = progress.get(id)
  if (current === undefined) {
    return progress
  }
  const misses = current.misses + 1
  return new Map(progress).set(id, {
    ...current,
    inflight: false,
    misses,
    status: misses >= UNREACHABLE_MISSES ? "unreachable" : current.status,
  })
}

const resetAcked = (
  progress: ReadonlyMap<BulbId, BulbProgress>,
): ReadonlyMap<BulbId, BulbProgress> => {
  const next = new Map(progress)
  for (const [id, entry] of next) {
    next.set(id, { ...entry, last: null })
  }
  return next
}

export { initialProgress, markInflight, missProgress, propertiesFor, resetAcked, settleProgress }
export type { BulbProgress, BulbSession }
