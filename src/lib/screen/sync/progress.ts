import type { Bulb, BulbId } from "@/lib/bulbs/schema"
import type { MiioError } from "@/lib/miio/error"
import type { PropertyParam } from "@/lib/miio/properties"
import type { MiioSession } from "@/lib/miio/session"
import type { DerivedColor } from "@/lib/screen/reducer"
import type { BulbSyncPhase } from "@/lib/screen/sync/status"

import { PIID_COLOR, PIID_TEMPERATURE, packColor, parseColor } from "@/lib/bulbs/schema"
import { propertiesWithValue } from "@/lib/miio/properties"
import { NEUTRAL_KELVIN, shouldPublish } from "@/lib/screen/reducer"

const UNREACHABLE_MISSES = 3

interface BulbProgress {
  readonly last: DerivedColor | null
  readonly rejected: DerivedColor | null
  readonly rejectedMessage: string | null
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
      rejected: null,
      rejectedMessage: null,
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

const withMiss = (
  progress: ReadonlyMap<BulbId, BulbProgress>,
  id: BulbId,
  current: BulbProgress,
): ReadonlyMap<BulbId, BulbProgress> => {
  const misses = current.misses + 1
  const status = misses >= UNREACHABLE_MISSES ? "unreachable" : current.status
  return new Map(progress).set(id, {
    ...current,
    inflight: false,
    misses,
    rejectedMessage: status === "unreachable" ? null : current.rejectedMessage,
    status,
  })
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
  if (error === null) {
    return new Map(progress).set(id, {
      ...current,
      inflight: false,
      last: target,
      rejected: null,
      rejectedMessage: null,
      misses: 0,
      status: "syncing",
    })
  }
  if (error.kind === "device") {
    return new Map(progress).set(id, {
      ...current,
      inflight: false,
      rejected: target,
      rejectedMessage: error.message,
      misses: 0,
      status: "syncing",
    })
  }
  return withMiss(progress, id, current)
}

const missProgress = (
  progress: ReadonlyMap<BulbId, BulbProgress>,
  id: BulbId,
): ReadonlyMap<BulbId, BulbProgress> => {
  const current = progress.get(id)
  return current === undefined ? progress : withMiss(progress, id, current)
}

const shouldSend = (target: DerivedColor, entry: BulbProgress): boolean => {
  if (entry.status !== "syncing" || entry.inflight) {
    return false
  }
  if (entry.rejected !== null && !shouldPublish(target, entry.rejected)) {
    return false
  }
  return shouldPublish(target, entry.last)
}

const resetAcked = (
  progress: ReadonlyMap<BulbId, BulbProgress>,
): ReadonlyMap<BulbId, BulbProgress> => {
  const next = new Map(progress)
  for (const [id, entry] of next) {
    next.set(id, { ...entry, last: null, rejected: null, rejectedMessage: null })
  }
  return next
}

export {
  initialProgress,
  markInflight,
  missProgress,
  propertiesFor,
  resetAcked,
  settleProgress,
  shouldSend,
}
export type { BulbProgress, BulbSession }
