import type { Lab, Lch } from "./color"
import type { PreparedFrame } from "./frame"

import { gamutMapOklch, hueDistance, kelvinToLab, kelvinToRgb, oklabToOklch, toHex } from "./color"

export type AlgorithmId = "dominant" | "saliency" | "histogram"

export interface Candidate {
  readonly algorithm: AlgorithmId
  readonly lab: Lab
  readonly lch: Lch
  readonly rgb: Rgb
  readonly hex: string
  readonly neutral: boolean
  readonly kelvin: number | null
  readonly capped: boolean
  readonly gamutClamped: boolean
  readonly meta: Readonly<Record<string, number | string | boolean>>
}

export interface RunOptions {
  readonly neutralThreshold: number
  readonly neutralKelvin: number
  readonly chromaCap: boolean
  readonly k: number
  readonly warmStart: boolean
  readonly outputLightness: number | null
  readonly capBrightness: "output" | "fixed" | "scene"
}

export interface AlgorithmState {
  centroids: readonly Lab[] | null
}

export const createState = (): AlgorithmState => ({ centroids: null })

const neutralCandidate = (
  algorithm: AlgorithmId,
  kelvin: number,
  frame: PreparedFrame,
  reason: string,
): Candidate => {
  const rgb = kelvinToRgb(kelvin)
  const lab = kelvinToLab(kelvin)
  return {
    algorithm,
    lab,
    lch: oklabToOklch(lab),
    rgb,
    hex: toHex(rgb),
    neutral: true,
    kelvin,
    capped: false,
    gamutClamped: false,
    meta: {
      reason,
      p90Chroma: frame.p90Chroma,
      p95Chroma: frame.p95Chroma,
      sceneBrightness: frame.meanL * 100,
    },
  }
}

const finalize = (
  algorithm: AlgorithmId,
  lab: Lab,
  frame: PreparedFrame,
  options: RunOptions,
  meta: Record<string, number | string | boolean>,
): Candidate => {
  const original = oklabToOklch(lab)
  const outputLightness = options.outputLightness ?? original.l
  const capBrightness =
    options.capBrightness === "fixed"
      ? 100
      : options.capBrightness === "scene"
        ? frame.meanL * 100
        : outputLightness * 100
  const maximumChroma = 0.04 + 0.0009 * capBrightness
  const capped = options.chromaCap
    ? { l: outputLightness, c: Math.min(original.c, maximumChroma), h: original.h }
    : { l: outputLightness, c: original.c, h: original.h }
  const mapped = gamutMapOklch(capped)
  return {
    algorithm,
    lab: {
      l: mapped.lch.l,
      a: mapped.lch.c * Math.cos((mapped.lch.h * Math.PI) / 180),
      b: mapped.lch.c * Math.sin((mapped.lch.h * Math.PI) / 180),
    },
    lch: mapped.lch,
    rgb: mapped.rgb,
    hex: toHex(mapped.rgb),
    neutral: false,
    kelvin: null,
    capped: capped.c !== original.c || outputLightness !== original.l,
    gamutClamped: mapped.clamped,
    meta: {
      ...meta,
      p90Chroma: frame.p90Chroma,
      p95Chroma: frame.p95Chroma,
      cappedChroma: capped.c,
      gamutClamped: mapped.clamped,
      outputLightness,
      capBrightness,
      sceneBrightness: frame.meanL * 100,
      winnerChroma: original.c,
    },
  }
}

const makeRng = (seed: number): (() => number) => {
  let state = seed >>> 0
  return () => {
    state = (state + 0x6d2b79f5) | 0
    let t = Math.imul(state ^ (state >>> 15), 1 | state)
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t
    return ((t ^ (t >>> 14)) >>> 0) / 4_294_967_296
  }
}

const labDistanceSquared = (frame: PreparedFrame, index: number, centroid: Lab): number => {
  const dl = (frame.lab[index * 3] ?? 0) - centroid.l
  const da = (frame.lab[index * 3 + 1] ?? 0) - centroid.a
  const db = (frame.lab[index * 3 + 2] ?? 0) - centroid.b
  return dl * dl + da * da + db * db
}

const initKmeansPlusPlus = (frame: PreparedFrame, k: number, seed: number): Lab[] => {
  const rng = makeRng(seed)
  const centroids: Lab[] = []
  const first = Math.min(frame.pixelCount - 1, Math.floor(rng() * frame.pixelCount))
  centroids.push({
    l: frame.lab[first * 3] ?? 0,
    a: frame.lab[first * 3 + 1] ?? 0,
    b: frame.lab[first * 3 + 2] ?? 0,
  })

  const distances = new Float32Array(frame.pixelCount)
  for (let cursor = 0; cursor < frame.pixelCount; cursor += 1) {
    distances[cursor] = labDistanceSquared(frame, cursor, centroids[0] as Lab)
  }

  while (centroids.length < k) {
    let total = 0
    for (let cursor = 0; cursor < frame.pixelCount; cursor += 1) {
      total += distances[cursor] ?? 0
    }
    if (total <= 0) {
      const index = Math.floor(rng() * frame.pixelCount)
      centroids.push({
        l: frame.lab[index * 3] ?? 0,
        a: frame.lab[index * 3 + 1] ?? 0,
        b: frame.lab[index * 3 + 2] ?? 0,
      })
      continue
    }
    let target = rng() * total
    let chosen = frame.pixelCount - 1
    for (let cursor = 0; cursor < frame.pixelCount; cursor += 1) {
      target -= distances[cursor] ?? 0
      if (target <= 0) {
        chosen = cursor
        break
      }
    }
    const centroid: Lab = {
      l: frame.lab[chosen * 3] ?? 0,
      a: frame.lab[chosen * 3 + 1] ?? 0,
      b: frame.lab[chosen * 3 + 2] ?? 0,
    }
    centroids.push(centroid)
    for (let cursor = 0; cursor < frame.pixelCount; cursor += 1) {
      const distance = labDistanceSquared(frame, cursor, centroid)
      if (distance < (distances[cursor] ?? 0)) {
        distances[cursor] = distance
      }
    }
  }
  return centroids
}

const runKmeans = (
  frame: PreparedFrame,
  centroids: Lab[],
  iterations: number,
): { centroids: Lab[]; assignments: Int32Array } => {
  const assignments = new Int32Array(frame.pixelCount)
  let current = centroids
  for (let iteration = 0; iteration < iterations; iteration += 1) {
    const sumL = new Float64Array(current.length)
    const sumA = new Float64Array(current.length)
    const sumB = new Float64Array(current.length)
    const count = new Int32Array(current.length)

    for (let cursor = 0; cursor < frame.pixelCount; cursor += 1) {
      let best = 0
      let bestDistance = Number.POSITIVE_INFINITY
      for (let cluster = 0; cluster < current.length; cluster += 1) {
        const distance = labDistanceSquared(frame, cursor, current[cluster] as Lab)
        if (distance < bestDistance) {
          bestDistance = distance
          best = cluster
        }
      }
      assignments[cursor] = best
      sumL[best] = (sumL[best] ?? 0) + (frame.lab[cursor * 3] ?? 0)
      sumA[best] = (sumA[best] ?? 0) + (frame.lab[cursor * 3 + 1] ?? 0)
      sumB[best] = (sumB[best] ?? 0) + (frame.lab[cursor * 3 + 2] ?? 0)
      count[best] = (count[best] ?? 0) + 1
    }

    let shift = 0
    const next: Lab[] = current.map((centroid, cluster) => {
      const size = count[cluster] ?? 0
      if (size === 0) {
        return centroid
      }
      const updated: Lab = {
        l: (sumL[cluster] ?? 0) / size,
        a: (sumA[cluster] ?? 0) / size,
        b: (sumB[cluster] ?? 0) / size,
      }
      shift = Math.max(
        shift,
        Math.hypot(updated.l - centroid.l, updated.a - centroid.a, updated.b - centroid.b),
      )
      return updated
    })
    current = next
    if (shift < 1e-5) {
      break
    }
  }
  return { centroids: current, assignments }
}

const dominant = (frame: PreparedFrame, options: RunOptions, state: AlgorithmState): Candidate => {
  const k = options.k
  const warm = options.warmStart && state.centroids !== null && state.centroids.length === k
  const initial = warm
    ? [...(state.centroids as readonly Lab[])]
    : initKmeansPlusPlus(frame, k, 0x9e3779b9)
  const { centroids, assignments } = runKmeans(frame, initial, 14)
  state.centroids = centroids

  const count = new Int32Array(k)
  const sumL = new Float64Array(k)
  const sumA = new Float64Array(k)
  const sumB = new Float64Array(k)
  const sumC = new Float64Array(k)
  for (let cursor = 0; cursor < frame.pixelCount; cursor += 1) {
    const cluster = assignments[cursor] ?? 0
    count[cluster] = (count[cluster] ?? 0) + 1
    sumL[cluster] = (sumL[cluster] ?? 0) + (frame.lab[cursor * 3] ?? 0)
    sumA[cluster] = (sumA[cluster] ?? 0) + (frame.lab[cursor * 3 + 1] ?? 0)
    sumB[cluster] = (sumB[cluster] ?? 0) + (frame.lab[cursor * 3 + 2] ?? 0)
    sumC[cluster] = (sumC[cluster] ?? 0) + (frame.chroma[cursor] ?? 0)
  }

  let winner = -1
  let winnerScore = -1
  let winnerPopulation = 0
  let winnerChroma = 0
  let largest = 0
  for (let cluster = 0; cluster < k; cluster += 1) {
    const population = (count[cluster] ?? 0) / frame.pixelCount
    const chroma = (sumC[cluster] ?? 0) / Math.max(1, count[cluster] ?? 1)
    if (population > winnerPopulation) {
      winnerPopulation = population
      largest = cluster
    }
    if (population < 0.03) {
      continue
    }
    const score = population ** 0.8 * (0.2 + 0.8 * Math.min(chroma / 0.15, 1))
    if (score > winnerScore) {
      winnerScore = score
      winner = cluster
      winnerChroma = chroma
    }
  }
  const chosen = winner >= 0 ? winner : largest
  const chosenCount = Math.max(1, count[chosen] ?? 1)
  const lab: Lab = {
    l: (sumL[chosen] ?? 0) / chosenCount,
    a: (sumA[chosen] ?? 0) / chosenCount,
    b: (sumB[chosen] ?? 0) / chosenCount,
  }
  const chosenChroma = (sumC[chosen] ?? 0) / chosenCount
  const hue = ((Math.atan2(lab.b, lab.a) * 180) / Math.PI + 360) % 360
  const output: Lab = {
    l: lab.l,
    a: chosenChroma * Math.cos((hue * Math.PI) / 180),
    b: chosenChroma * Math.sin((hue * Math.PI) / 180),
  }
  return finalize("dominant", output, frame, options, {
    winner: chosen,
    winnerPopulation: (count[chosen] ?? 0) / frame.pixelCount,
    winnerChroma: chosenChroma,
    winnerScore,
    largestPopulation: (count[largest] ?? 0) / frame.pixelCount,
    populationFallback: winner < 0,
    warmStart: warm,
  })
}

const boxBlur = (
  values: Float32Array,
  width: number,
  height: number,
  radius: number,
): Float32Array => {
  const horizontal = new Float32Array(values.length)
  const output = new Float32Array(values.length)
  const window = radius * 2 + 1

  for (let y = 0; y < height; y += 1) {
    let sum = 0
    for (let x = -radius; x <= radius; x += 1) {
      sum += values[y * width + Math.max(0, Math.min(width - 1, x))] ?? 0
    }
    for (let x = 0; x < width; x += 1) {
      horizontal[y * width + x] = sum / window
      const remove = values[y * width + Math.max(0, x - radius)] ?? 0
      const add = values[y * width + Math.min(width - 1, x + radius + 1)] ?? 0
      sum += add - remove
    }
  }

  for (let x = 0; x < width; x += 1) {
    let sum = 0
    for (let y = -radius; y <= radius; y += 1) {
      sum += horizontal[Math.max(0, Math.min(height - 1, y)) * width + x] ?? 0
    }
    for (let y = 0; y < height; y += 1) {
      output[y * width + x] = sum / window
      const remove = horizontal[Math.max(0, y - radius) * width + x] ?? 0
      const add = horizontal[Math.min(height - 1, y + radius + 1) * width + x] ?? 0
      sum += add - remove
    }
  }
  return output
}

const saliencyBackground = (frame: PreparedFrame, options: RunOptions): Candidate => {
  const blurred = boxBlur(frame.luma, frame.rect.width, frame.rect.height, 3)
  const saliency = new Float32Array(frame.pixelCount)
  for (let cursor = 0; cursor < frame.pixelCount; cursor += 1) {
    saliency[cursor] =
      Math.abs((frame.luma[cursor] ?? 0) - (blurred[cursor] ?? 0)) +
      0.35 * (frame.chroma[cursor] ?? 0)
  }
  const sorted = Array.from(saliency).sort((a, b) => a - b)
  const cutoff = sorted[Math.floor(sorted.length * 0.1)] ?? 0
  let count = 0
  let sumL = 0
  let sumA = 0
  let sumB = 0
  for (let cursor = 0; cursor < frame.pixelCount; cursor += 1) {
    if ((saliency[cursor] ?? 0) <= cutoff) {
      count += 1
      sumL += frame.lab[cursor * 3] ?? 0
      sumA += frame.lab[cursor * 3 + 1] ?? 0
      sumB += frame.lab[cursor * 3 + 2] ?? 0
    }
  }
  const size = Math.max(1, count)
  const lab: Lab = { l: sumL / size, a: sumA / size, b: sumB / size }
  return finalize("saliency", lab, frame, options, {
    cutoff,
    pixels: count,
    fraction: count / frame.pixelCount,
  })
}

const chromaBin = (chroma: number): number => {
  const edges = [0.02, 0.04, 0.07, 0.11, 0.16]
  for (let index = 0; index < edges.length; index += 1) {
    if (chroma < (edges[index] as number)) {
      return index
    }
  }
  return edges.length
}

const hueHistogram = (frame: PreparedFrame, options: RunOptions): Candidate => {
  const hueBins = 36
  const chromaBins = 6
  const lightnessBins = 6
  const binCount = hueBins * chromaBins * lightnessBins
  const weight = new Float64Array(binCount)
  const sumL = new Float64Array(binCount)
  const sumA = new Float64Array(binCount)
  const sumB = new Float64Array(binCount)

  for (let cursor = 0; cursor < frame.pixelCount; cursor += 1) {
    const chroma = frame.chroma[cursor] ?? 0
    if (chroma < 0.004) {
      continue
    }
    const hue = frame.hue[cursor] ?? 0
    const lightness = frame.lab[cursor * 3] ?? 0
    const hueIndex = Math.min(hueBins - 1, Math.floor((hue / 360) * hueBins))
    const chromaIndex = chromaBin(chroma)
    const lightnessIndex = Math.max(
      0,
      Math.min(lightnessBins - 1, Math.floor(lightness * lightnessBins)),
    )
    const bin = (lightnessIndex * chromaBins + chromaIndex) * hueBins + hueIndex
    weight[bin] = (weight[bin] ?? 0) + chroma
    sumL[bin] = (sumL[bin] ?? 0) + (frame.lab[cursor * 3] ?? 0) * chroma
    sumA[bin] = (sumA[bin] ?? 0) + (frame.lab[cursor * 3 + 1] ?? 0) * chroma
    sumB[bin] = (sumB[bin] ?? 0) + (frame.lab[cursor * 3 + 2] ?? 0) * chroma
  }

  let best = -1
  let bestWeight = 0
  let totalWeight = 0
  for (let bin = 0; bin < binCount; bin += 1) {
    const value = weight[bin] ?? 0
    totalWeight += value
    if (value > bestWeight) {
      bestWeight = value
      best = bin
    }
  }
  if (best < 0 || bestWeight <= 0) {
    return neutralCandidate("histogram", options.neutralKelvin, frame, "empty-histogram")
  }
  const lab: Lab = {
    l: (sumL[best] ?? 0) / bestWeight,
    a: (sumA[best] ?? 0) / bestWeight,
    b: (sumB[best] ?? 0) / bestWeight,
  }
  const hueIndex = best % hueBins
  return finalize("histogram", lab, frame, options, {
    hueBin: hueIndex,
    hueCenter: hueIndex * 10 + 5,
    binWeight: bestWeight,
    binShare: bestWeight / Math.max(1e-9, totalWeight),
  })
}

export const runAlgorithm = (
  algorithm: AlgorithmId,
  frame: PreparedFrame,
  options: RunOptions,
  state: AlgorithmState,
): Candidate => {
  if (frame.p90Chroma < options.neutralThreshold) {
    return neutralCandidate(algorithm, options.neutralKelvin, frame, "p90-chroma")
  }
  if (algorithm === "dominant") {
    return dominant(frame, options, state)
  }
  if (algorithm === "saliency") {
    return saliencyBackground(frame, options)
  }
  return hueHistogram(frame, options)
}

export const candidateHue = (candidate: Candidate): number => candidate.lch.h

export const candidateDelta = (a: Candidate, b: Candidate): number => {
  if (a.neutral && b.neutral) {
    return 0
  }
  return Math.hypot(a.lab.l - b.lab.l, a.lab.a - b.lab.a, a.lab.b - b.lab.b)
}

export const candidateHueShift = (a: Candidate, b: Candidate): number => {
  if (a.neutral || b.neutral) {
    return 0
  }
  return hueDistance(a.lch.h, b.lch.h)
}
