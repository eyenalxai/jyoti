import type { AlgorithmId, Candidate, RunOptions } from "./algorithms"
import type { PreparedFrame } from "./frame"

import { candidateDelta, candidateHueShift, createState, runAlgorithm } from "./algorithms"
import { gamutMapOklch, toHex } from "./color"

export interface TickRecord {
  readonly index: number
  readonly timeMs: number
  readonly raw: Candidate
  readonly smooth: Candidate
  readonly published: Candidate
  readonly costMs: number
}

export interface RawTick {
  readonly raw: Candidate
  readonly costMs: number
}

export interface SequenceOptions extends RunOptions {
  readonly deadband: number
  readonly smoothingTauMs: number
}

export const collectRaw = (
  algorithm: AlgorithmId,
  frames: readonly PreparedFrame[],
  options: RunOptions,
): readonly RawTick[] => {
  const state = createState()
  const ticks: RawTick[] = []
  for (const frame of frames) {
    const start = performance.now()
    const raw = runAlgorithm(algorithm, frame, options, state)
    ticks.push({ raw, costMs: performance.now() - start })
  }
  return ticks
}

export const simulateTemporal = (
  rawTicks: readonly RawTick[],
  smoothingTauMs: number,
  deadband: number,
): readonly TickRecord[] => {
  const records: TickRecord[] = []
  let previousSmooth: Candidate | null = null
  let published: Candidate | null = null
  for (let index = 0; index < rawTicks.length; index += 1) {
    const tick = rawTicks[index] as RawTick
    const smooth = smoothLab(previousSmooth, tick.raw, smoothingTauMs, 666.6667)
    previousSmooth = smooth
    if (shouldPublish(published, smooth, deadband)) {
      published = smooth
    }
    records.push({
      index,
      timeMs: index * 666.6667,
      raw: tick.raw,
      smooth,
      published: published as Candidate,
      costMs: tick.costMs,
    })
  }
  return records
}

const smoothLab = (
  previous: Candidate | null,
  current: Candidate,
  tauMs: number,
  dtMs: number,
): Candidate => {
  if (current.neutral || previous === null || previous.neutral) {
    return current
  }
  const alpha = 1 - Math.exp(-dtMs / tauMs)
  const lab = {
    l: previous.lab.l + (current.lab.l - previous.lab.l) * alpha,
    a: previous.lab.a + (current.lab.a - previous.lab.a) * alpha,
    b: previous.lab.b + (current.lab.b - previous.lab.b) * alpha,
  }
  const lch = {
    l: lab.l,
    c: Math.hypot(lab.a, lab.b),
    h: ((Math.atan2(lab.b, lab.a) * 180) / Math.PI + 360) % 360,
  }
  const mapped = gamutMapOklch(lch)
  return {
    ...current,
    lab: {
      l: mapped.lch.l,
      a: mapped.lch.c * Math.cos((mapped.lch.h * Math.PI) / 180),
      b: mapped.lch.c * Math.sin((mapped.lch.h * Math.PI) / 180),
    },
    lch: mapped.lch,
    rgb: mapped.rgb,
    hex: toHex(mapped.rgb),
  }
}

const shouldPublish = (previous: Candidate | null, next: Candidate, deadband: number): boolean => {
  if (previous === null) {
    return true
  }
  if (previous.neutral !== next.neutral) {
    return true
  }
  if (previous.neutral && next.neutral) {
    return previous.kelvin !== next.kelvin
  }
  return candidateDelta(previous, next) >= deadband
}

export const runSequence = (
  algorithm: AlgorithmId,
  frames: readonly PreparedFrame[],
  options: SequenceOptions,
): readonly TickRecord[] =>
  simulateTemporal(collectRaw(algorithm, frames, options), options.smoothingTauMs, options.deadband)

export interface SequenceMetrics {
  readonly algorithm: AlgorithmId
  readonly frames: number
  readonly neutralRate: number
  readonly rawJitter: number
  readonly smoothJitter: number
  readonly publishedJitter: number
  readonly publishedChanges: number
  readonly publishRate: number
  readonly deadbandSuppressed: number
  readonly neutralFlips: number
  readonly hueFlips: number
  readonly meanL: number
  readonly meanC: number
  readonly meanCostMs: number
  readonly maxCostMs: number
  readonly neutralKelvin: number | null
}

const mean = (values: readonly number[]): number =>
  values.length === 0 ? 0 : values.reduce((sum, value) => sum + value, 0) / values.length

export const sequenceMetrics = (
  algorithm: AlgorithmId,
  records: readonly TickRecord[],
  deadband: number,
): SequenceMetrics => {
  let rawJitter = 0
  let rawPairs = 0
  let smoothJitter = 0
  let smoothPairs = 0
  let publishedJitter = 0
  let publishedPairs = 0
  let publishedChanges = 0
  let suppressed = 0
  let neutralFlips = 0
  let hueFlips = 0
  let previousRaw: Candidate | null = null
  let previousSmooth: Candidate | null = null
  let previousPublished: Candidate | null = null

  for (const record of records) {
    if (previousRaw !== null && !previousRaw.neutral && !record.raw.neutral) {
      rawJitter += candidateDelta(previousRaw, record.raw)
      rawPairs += 1
    }
    if (previousSmooth !== null && !previousSmooth.neutral && !record.smooth.neutral) {
      smoothJitter += candidateDelta(previousSmooth, record.smooth)
      smoothPairs += 1
    }
    if (previousPublished !== null) {
      if (previousPublished.neutral !== record.published.neutral) {
        neutralFlips += 1
      } else if (
        !record.published.neutral &&
        candidateHueShift(previousPublished, record.published) > 20
      ) {
        hueFlips += 1
      }
      if (!previousPublished.neutral && !record.published.neutral) {
        publishedJitter += candidateDelta(previousPublished, record.published)
        publishedPairs += 1
      }
      if (
        candidateDelta(previousPublished, record.published) < deadband &&
        previousPublished.neutral === record.published.neutral
      ) {
        suppressed += 1
      } else {
        publishedChanges += 1
      }
    }
    previousRaw = record.raw
    previousSmooth = record.smooth
    previousPublished = record.published
  }

  const nonNeutral = records.filter((record) => !record.raw.neutral)
  const neutral = records.filter((record) => record.raw.neutral)
  return {
    algorithm,
    frames: records.length,
    neutralRate: neutral.length / Math.max(1, records.length),
    rawJitter: rawJitter / Math.max(1, rawPairs),
    smoothJitter: smoothJitter / Math.max(1, smoothPairs),
    publishedJitter: publishedJitter / Math.max(1, publishedPairs),
    publishedChanges: records.length > 0 ? publishedChanges + 1 : 0,
    publishRate: (publishedChanges + 1) / Math.max(1, records.length),
    deadbandSuppressed: suppressed,
    neutralFlips,
    hueFlips,
    meanL: mean(nonNeutral.map((record) => record.raw.lch.l)),
    meanC: mean(nonNeutral.map((record) => record.raw.lch.c)),
    meanCostMs: mean(records.map((record) => record.costMs)),
    maxCostMs: Math.max(0, ...records.map((record) => record.costMs)),
    neutralKelvin: neutral.length > 0 ? (neutral[0]?.raw.kelvin ?? null) : null,
  }
}
