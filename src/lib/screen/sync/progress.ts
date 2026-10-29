import type { Bulb, BulbId, BulbState } from "@/lib/bulbs/schema"
import type { MiioError } from "@/lib/miio/error"
import type { PropertyParam } from "@/lib/miio/properties"
import type { DerivedColor } from "@/lib/screen/reducer"
import type { BulbSyncPhase } from "@/lib/screen/sync/status"

import { PIID_BRIGHTNESS, PIID_COLOR, packColor } from "@/lib/bulbs/schema"
import { propertiesWithValue } from "@/lib/miio/properties"
import { shouldPublish } from "@/lib/screen/reducer"
import { slewBrightness } from "@/lib/screen/sync/brightness"

const UNREACHABLE_MISSES = 3

interface BulbProgress {
  readonly last: DerivedColor | null
  readonly lastBrightness: number | null
  readonly rejected: DerivedColor | null
  readonly rejectedTarget: number | null
  readonly rejectedMessage: string | null
  readonly inflight: boolean
  readonly misses: number
  readonly status: BulbSyncPhase
  readonly power: boolean | null
  readonly appliedBrightness: number | null
  readonly droveBrightness: boolean
  readonly probeDueAt: number | null
}

interface SendPlan {
  readonly brightness: number | null
  readonly publishColor: boolean
  readonly targetBrightness: number
}

const propertiesFor = (target: DerivedColor, plan: SendPlan): readonly PropertyParam[] => {
  const control = plan.publishColor ? propertiesWithValue(PIID_COLOR, packColor(target.rgb)) : []
  const brightnessProperties =
    plan.brightness === null ? [] : propertiesWithValue(PIID_BRIGHTNESS, plan.brightness)
  return [...control, ...brightnessProperties]
}

const initialProgress = (
  bulbs: readonly Bulb[],
  reachable: ReadonlySet<BulbId>,
  states: ReadonlyMap<BulbId, BulbState>,
): Map<BulbId, BulbProgress> => {
  const progress = new Map<BulbId, BulbProgress>()
  for (const bulb of bulbs) {
    const state = states.get(bulb.id)
    const power = state?.power ?? null
    const brightness = state?.brightness ?? null
    progress.set(bulb.id, {
      last: null,
      lastBrightness: null,
      rejected: null,
      rejectedTarget: null,
      rejectedMessage: null,
      inflight: false,
      misses: 0,
      status: reachable.has(bulb.id) ? "syncing" : "unreachable",
      power,
      appliedBrightness: power === true && brightness !== null ? brightness : null,
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
  plan: SendPlan,
  error: MiioError | null,
): ReadonlyMap<BulbId, BulbProgress> => {
  if (error === null) {
    return updateEntry(progress, id, (current) => ({
      ...current,
      inflight: false,
      last: plan.publishColor ? target : current.last,
      lastBrightness: plan.brightness ?? current.lastBrightness,
      appliedBrightness: plan.brightness ?? current.appliedBrightness,
      rejected: null,
      rejectedTarget: null,
      rejectedMessage: null,
      misses: 0,
      status: "syncing",
      droveBrightness: current.droveBrightness || plan.brightness !== null,
    }))
  }
  if (error.kind === "device") {
    return updateEntry(progress, id, (current) => ({
      ...current,
      inflight: false,
      rejected: plan.publishColor ? target : current.rejected,
      rejectedTarget: plan.brightness === null ? current.rejectedTarget : plan.targetBrightness,
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

const planSend = (
  entry: BulbProgress,
  target: DerivedColor,
  targetBrightness: number,
  flashHold: boolean,
): SendPlan | null => {
  if (entry.status !== "syncing" || entry.inflight) {
    return null
  }
  const applied =
    entry.power === true && !flashHold
      ? slewBrightness(entry.appliedBrightness, targetBrightness)
      : null
  const targetRejected = entry.rejectedTarget === targetBrightness
  const nextBrightness =
    entry.power === true && applied !== null && applied !== entry.lastBrightness ? applied : null
  const brightness = targetRejected ? null : nextBrightness
  const colorRejected = entry.rejected !== null && !shouldPublish(target, entry.rejected)
  const publishColor = !colorRejected && shouldPublish(target, entry.last)
  if (!publishColor && brightness === null) {
    return null
  }
  return { brightness, publishColor, targetBrightness }
}

const resetAcked = (
  progress: ReadonlyMap<BulbId, BulbProgress>,
): ReadonlyMap<BulbId, BulbProgress> => {
  const next = new Map(progress)
  for (const [id, entry] of next) {
    next.set(id, {
      ...entry,
      last: null,
      lastBrightness: null,
      rejected: null,
      rejectedTarget: null,
      rejectedMessage: null,
    })
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
    lastBrightness: null,
    rejected: null,
    rejectedTarget: null,
    rejectedMessage: null,
    misses: 0,
    status: "syncing",
    probeDueAt: null,
  }))

const setPower = (
  progress: ReadonlyMap<BulbId, BulbProgress>,
  id: BulbId,
  power: boolean | null,
): ReadonlyMap<BulbId, BulbProgress> =>
  updateEntry(progress, id, (entry) => {
    if (entry.power === power) {
      return entry
    }
    const keepApplied = entry.power !== false && power !== false
    const turnedOn = power === true && entry.power !== true
    return {
      ...entry,
      power,
      appliedBrightness: keepApplied ? entry.appliedBrightness : null,
      lastBrightness: keepApplied ? entry.lastBrightness : null,
      rejected: turnedOn ? null : entry.rejected,
      rejectedTarget: turnedOn ? null : entry.rejectedTarget,
      rejectedMessage: turnedOn ? null : entry.rejectedMessage,
    }
  })

const setPowers = (
  progress: ReadonlyMap<BulbId, BulbProgress>,
  powers: ReadonlyMap<BulbId, boolean | null>,
): ReadonlyMap<BulbId, BulbProgress> => {
  let next = progress
  for (const [id, power] of powers) {
    next = setPower(next, id, power)
  }
  return next
}

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
  planSend,
  probeDue,
  propertiesFor,
  resetAcked,
  setPowers,
  settleProgress,
}
export type { BulbProgress, SendPlan }
