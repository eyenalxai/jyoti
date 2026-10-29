import type { ColorHex } from "@/lib/bulbs/schema"
import type { ScreenFrame } from "@/lib/screen/frame"
import type { PreparedFrame } from "@/lib/screen/grid"
import type { Lab } from "@/lib/screen/lab"

import { kelvinToSrgb, oklchToSrgb } from "@/lib/color"
import { prepareFrame } from "@/lib/screen/grid"
import { cluster } from "@/lib/screen/kmeans"
import { deltaEOk, labFromOklch, mixLab, oklchFromLab } from "@/lib/screen/lab"

const NEUTRAL_ENTER_CHROMA = 0.03
const NEUTRAL_EXIT_CHROMA = 0.045
const NEUTRAL_KELVIN = 4000
const OUTPUT_LIGHTNESS = 0.65
const OUTPUT_CHROMA_CAP = 0.13
const COLOR_TAU_MS = 450
const SCENE_TAU_MS = 900
const CLUSTER_DROP = 0.03
const SCORE_CHROMA_SCALE = 0.15
const DEADBAND = 0.02

interface ReducerState {
  readonly centroids: readonly Lab[] | null
  readonly neutral: boolean
  readonly primed: boolean
  readonly color: Lab | null
  readonly sceneL: number | null
}

interface DerivedColor {
  readonly mode: "color" | "temperature"
  readonly hex: ColorHex
  readonly kelvin: number | null
  readonly lab: Lab
  readonly sceneL: number
}

interface Reduction {
  readonly derived: DerivedColor
  readonly state: ReducerState
  readonly p90Chroma: number
}

const createReducerState = (): ReducerState => ({
  centroids: null,
  neutral: false,
  primed: false,
  color: null,
  sceneL: null,
})

const smoothing = (tauMs: number, elapsedMs: number): number =>
  1 - Math.exp(-Math.max(0, elapsedMs) / tauMs)

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
  let sumL = 0
  let sumA = 0
  let sumB = 0
  let chosenCount = 0
  for (let cursor = 0; cursor < prepared.pixelCount; cursor += 1) {
    if ((assignments[cursor] ?? 0) !== winner.index) {
      continue
    }
    sumL += prepared.lab[cursor * 3] ?? 0
    sumA += prepared.lab[cursor * 3 + 1] ?? 0
    sumB += prepared.lab[cursor * 3 + 2] ?? 0
    chosenCount += 1
  }

  const size = Math.max(1, chosenCount)
  const hue = Math.atan2(sumB / size, sumA / size)
  const chroma = Math.min(winner.chroma, OUTPUT_CHROMA_CAP)
  return {
    l: OUTPUT_LIGHTNESS,
    a: chroma * Math.cos(hue),
    b: chroma * Math.sin(hue),
  }
}

const temperatureFallback = (sceneL: number): DerivedColor => {
  const fallback = kelvinToSrgb(NEUTRAL_KELVIN)
  return {
    mode: "temperature",
    hex: fallback.hex,
    kelvin: NEUTRAL_KELVIN,
    lab: labFromOklch(fallback.oklch),
    sceneL,
  }
}

const reducePrepared = (
  prepared: PreparedFrame,
  state: ReducerState,
  elapsedMs: number,
): Reduction => {
  const neutral = state.primed
    ? state.neutral
      ? prepared.p90Chroma <= NEUTRAL_EXIT_CHROMA
      : prepared.p90Chroma < NEUTRAL_ENTER_CHROMA
    : prepared.p90Chroma < NEUTRAL_ENTER_CHROMA

  const sceneAlpha = state.sceneL === null ? 1 : smoothing(SCENE_TAU_MS, elapsedMs)
  const sceneL =
    state.sceneL === null
      ? prepared.meanL
      : state.sceneL + (prepared.meanL - state.sceneL) * sceneAlpha

  if (neutral) {
    const derived = temperatureFallback(sceneL)
    return {
      derived,
      state: {
        centroids: state.centroids,
        neutral: true,
        primed: true,
        color: derived.lab,
        sceneL,
      },
      p90Chroma: prepared.p90Chroma,
    }
  }

  const clustering = cluster(prepared.lab, prepared.pixelCount, state.centroids)
  const target = chooseTarget(prepared, clustering.assignments, clustering.centroids.length)
  const targetSrgb = oklchToSrgb(oklchFromLab(target))
  const targetLab = labFromOklch(targetSrgb.oklch)
  const alpha = state.primed && !state.neutral ? smoothing(COLOR_TAU_MS, elapsedMs) : 1
  const smoothed =
    state.color === null || alpha >= 1 ? targetLab : mixLab(state.color, targetLab, alpha)
  const srgb = oklchToSrgb(oklchFromLab(smoothed))
  const lab = labFromOklch(srgb.oklch)

  return {
    derived: {
      mode: "color",
      hex: srgb.hex,
      kelvin: null,
      lab,
      sceneL,
    },
    state: {
      centroids: clustering.centroids,
      neutral: false,
      primed: true,
      color: lab,
      sceneL,
    },
    p90Chroma: prepared.p90Chroma,
  }
}

const reduceFrame = (frame: ScreenFrame, state: ReducerState, elapsedMs: number): Reduction =>
  reducePrepared(prepareFrame(frame), state, elapsedMs)

const shouldPublish = (
  next: DerivedColor,
  last: Pick<DerivedColor, "mode" | "lab"> | null,
): boolean => {
  if (last === null) {
    return true
  }
  if (last.mode !== next.mode) {
    return true
  }
  return deltaEOk(next.lab, last.lab) > DEADBAND
}

export { createReducerState, reduceFrame, reducePrepared, shouldPublish }
export type { DerivedColor, Reduction, ReducerState }
