import type { ColorHex, Rgb } from "@/lib/bulbs/schema"
import type { ScreenFrame } from "@/lib/screen/frame"
import type { PreparedFrame } from "@/lib/screen/grid"
import type { Lab } from "@/lib/screen/lab"

import { oklchToSrgb } from "@/lib/color"
import { percentile } from "@/lib/format"
import { prepareFrame } from "@/lib/screen/grid"
import { cluster } from "@/lib/screen/kmeans"
import { deltaEOk, labFromOklch, oklchFromLab } from "@/lib/screen/lab"

const CHROMA_QUANTILE = 0.75
const CLUSTER_DROP = 0.03
const SCORE_CHROMA_SCALE = 0.15
const DEADBAND = 0.02

interface ReducerState {
  readonly centroids: readonly Lab[] | null
}

interface DerivedColor {
  readonly hex: ColorHex
  readonly rgb: Rgb
  readonly lab: Lab
  readonly sceneBrightness: number
}

interface Reduction {
  readonly derived: DerivedColor
  readonly state: ReducerState
  readonly p90Chroma: number
}

const createReducerState = (): ReducerState => ({
  centroids: null,
})

const pickWinner = (
  counts: Int32Array,
  chromaSums: Float64Array,
  pixelCount: number,
): { readonly index: number; readonly chroma: number } => {
  let winner = -1
  let winnerScore = -1
  let largest = 0
  let largestPopulation = -1
  for (let index = 0; index < counts.length; index += 1) {
    const count = counts[index] ?? 0
    const population = count / Math.max(1, pixelCount)
    if (population > largestPopulation) {
      largestPopulation = population
      largest = index
    }
    if (population < CLUSTER_DROP) {
      continue
    }
    const chroma = (chromaSums[index] ?? 0) / Math.max(1, count)
    const score = population ** 0.8 * (0.2 + 0.8 * Math.min(chroma / SCORE_CHROMA_SCALE, 1))
    if (score > winnerScore) {
      winnerScore = score
      winner = index
    }
  }
  const chosen = winner >= 0 ? winner : largest
  return {
    index: chosen,
    chroma: (chromaSums[chosen] ?? 0) / Math.max(1, counts[chosen] ?? 0),
  }
}

const chooseTarget = (
  prepared: PreparedFrame,
  assignments: Int32Array,
  centroidCount: number,
): Lab => {
  const counts = new Int32Array(centroidCount)
  const chromaSums = new Float64Array(centroidCount)
  for (let cursor = 0; cursor < prepared.pixelCount; cursor += 1) {
    const clusterIndex = assignments[cursor] ?? 0
    counts[clusterIndex] = (counts[clusterIndex] ?? 0) + 1
    chromaSums[clusterIndex] = (chromaSums[clusterIndex] ?? 0) + (prepared.chroma[cursor] ?? 0)
  }

  const winner = pickWinner(counts, chromaSums, prepared.pixelCount)
  const chromas = new Float32Array(counts[winner.index] ?? 0)
  let sumL = 0
  let sumA = 0
  let sumB = 0
  let chosen = 0
  for (let index = 0; index < prepared.pixelCount; index += 1) {
    if ((assignments[index] ?? 0) !== winner.index) {
      continue
    }
    sumL += prepared.lab[index * 3] ?? 0
    sumA += prepared.lab[index * 3 + 1] ?? 0
    sumB += prepared.lab[index * 3 + 2] ?? 0
    chromas[chosen] = prepared.chroma[index] ?? 0
    chosen += 1
  }

  const size = Math.max(1, chosen)
  const scale = winner.chroma > 0 ? percentile(chromas, CHROMA_QUANTILE) / winner.chroma : 0
  return {
    l: sumL / size,
    a: (sumA / size) * scale,
    b: (sumB / size) * scale,
  }
}

const reducePrepared = (prepared: PreparedFrame, state: ReducerState): Reduction => {
  const clustering = cluster(prepared.lab, prepared.pixelCount, state.centroids)
  const target = chooseTarget(prepared, clustering.assignments, clustering.centroids.length)
  const srgb = oklchToSrgb(oklchFromLab(target))
  const lab = labFromOklch(srgb.oklch)

  return {
    derived: {
      hex: srgb.hex,
      rgb: srgb.rgb,
      lab,
      sceneBrightness: prepared.meanDrive,
    },
    state: {
      centroids: clustering.centroids,
    },
    p90Chroma: prepared.p90Chroma,
  }
}

const reduceFrame = (frame: ScreenFrame, state: ReducerState): Reduction =>
  reducePrepared(prepareFrame(frame), state)

const shouldPublish = (next: DerivedColor, last: Pick<DerivedColor, "lab"> | null): boolean => {
  if (last === null) {
    return true
  }
  return deltaEOk(next.lab, last.lab) > DEADBAND
}

export { createReducerState, reduceFrame, reducePrepared, shouldPublish }
export type { DerivedColor, Reduction, ReducerState }
