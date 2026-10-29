import type { Bulb, BulbId } from "@/lib/bulbs/schema"
import type { MiioError } from "@/lib/miio/error"
import type { PropertyParam } from "@/lib/miio/properties"
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
  readonly droveBrightness: boolean
  readonly probeDueAt: number | null
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
      droveBrightness: false,
      probeDueAt: null,
    })
  }
  return progress
}

const updateEntry = (
  progress: ReadonlyMap<BulbId, BulbProgress>,
  id: BulbId,
  update: (entry: BulbProgress) => BulbProgress,
): ReadonlyMap<BulbId, BulbProgress> => {
  const current = progress.get(id)
  if (current === undefined) {
    return progress
  }
  return new Map(progress).set(id, update(current))
}

const markInflight = (
  progress: ReadonlyMap<BulbId, BulbProgress>,
  id: BulbId,
): ReadonlyMap<BulbId, BulbProgress> =>
  updateEntry(progress, id, (entry) => ({ ...entry, inflight: true }))

const withMiss = (
  progress: ReadonlyMap<BulbId, BulbProgress>,
  id: BulbId,
): ReadonlyMap<BulbId, BulbProgress> =>
  updateEntry(progress, id, (current) => {
    const misses = current.misses + 1
    const unreachable = misses >= UNREACHABLE_MISSES
    return {
      ...current,
      inflight: false,
      misses,
      rejectedMessage: unreachable ? null : current.rejectedMessage,
      status: unreachable ? "unreachable" : current.status,
      probeDueAt: unreachable ? null : current.probeDueAt,
    }
  })

const settleProgress = (
  progress: ReadonlyMap<BulbId, BulbProgress>,
  id: BulbId,
  target: DerivedColor,
  error: MiioError | null,
): ReadonlyMap<BulbId, BulbProgress> => {
  if (error === null) {
    return updateEntry(progress, id, (current) => ({
      ...current,
      inflight: false,
      last: target,
      rejected: null,
      rejectedMessage: null,
      misses: 0,
      status: "syncing",
    }))
  }
  if (error.kind === "device") {
    return updateEntry(progress, id, (current) => ({
      ...current,
      inflight: false,
      rejected: target,
      rejectedMessage: error.message,
      misses: 0,
      status: "syncing",
    }))
  }
  return withMiss(progress, id)
}

const missProgress = (
  progress: ReadonlyMap<BulbId, BulbProgress>,
  id: BulbId,
): ReadonlyMap<BulbId, BulbProgress> => withMiss(progress, id)

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

const probeDue = (entry: BulbProgress, now: number): boolean =>
  entry.status === "unreachable" && (entry.probeDueAt === null || now >= entry.probeDueAt)

const markProbe = (
  progress: ReadonlyMap<BulbId, BulbProgress>,
  id: BulbId,
  dueAt: number,
): ReadonlyMap<BulbId, BulbProgress> =>
  updateEntry(progress, id, (entry) => ({ ...entry, probeDueAt: dueAt }))

const markReachable = (
  progress: ReadonlyMap<BulbId, BulbProgress>,
  id: BulbId,
): ReadonlyMap<BulbId, BulbProgress> =>
  updateEntry(progress, id, (entry) => ({
    ...entry,
    last: null,
    rejected: null,
    rejectedMessage: null,
    misses: 0,
    status: "syncing",
    probeDueAt: null,
  }))

const markRestored = (
  progress: ReadonlyMap<BulbId, BulbProgress>,
  id: BulbId,
  status: "restored" | "not-restored",
): ReadonlyMap<BulbId, BulbProgress> =>
  updateEntry(progress, id, (entry) => ({ ...entry, inflight: false, status }))

export {
  initialProgress,
  markInflight,
  markProbe,
  markReachable,
  markRestored,
  missProgress,
  probeDue,
  propertiesFor,
  resetAcked,
  settleProgress,
  shouldSend,
}
export type { BulbProgress }
