import { Array as Arr } from "effect"

import type { Lab } from "@/lib/screen/lab"

const CLUSTERS = 5
const MAX_ITERATIONS = 14
const CONVERGENCE = 1e-5
const SEED = 2_654_435_761
const MULTIPLIER = 1_664_525
const INCREMENT = 1_013_904_223
const MODULUS = 4_294_967_296

interface Clustering {
  readonly centroids: readonly Lab[]
  readonly assignments: Int32Array
}

const makeRandom = (seed: number): (() => number) => {
  let state = seed
  return () => {
    state = (state * MULTIPLIER + INCREMENT) % MODULUS
    return state / MODULUS
  }
}

const labAt = (lab: Float32Array, index: number): Lab => ({
  l: lab[index * 3] ?? 0,
  a: lab[index * 3 + 1] ?? 0,
  b: lab[index * 3 + 2] ?? 0,
})

const distanceSquared = (lab: Float32Array, index: number, centroid: Lab): number => {
  const dl = (lab[index * 3] ?? 0) - centroid.l
  const da = (lab[index * 3 + 1] ?? 0) - centroid.a
  const db = (lab[index * 3 + 2] ?? 0) - centroid.b
  return dl * dl + da * da + db * db
}

const initCentroids = (
  lab: Float32Array,
  pixelCount: number,
  random: () => number,
): readonly Lab[] => {
  const firstIndex = Math.min(pixelCount - 1, Math.floor(random() * pixelCount))
  const first = labAt(lab, firstIndex)
  const centroids: Lab[] = [first]
  const distances = new Float32Array(pixelCount)
  for (let cursor = 0; cursor < pixelCount; cursor += 1) {
    distances[cursor] = distanceSquared(lab, cursor, first)
  }

  while (centroids.length < CLUSTERS) {
    let total = 0
    for (let cursor = 0; cursor < pixelCount; cursor += 1) {
      total += distances[cursor] ?? 0
    }
    if (total <= 0) {
      centroids.push(labAt(lab, Math.floor(random() * pixelCount)))
      continue
    }
    let target = random() * total
    let chosen = pixelCount - 1
    for (let cursor = 0; cursor < pixelCount; cursor += 1) {
      target -= distances[cursor] ?? 0
      if (target <= 0) {
        chosen = cursor
        break
      }
    }
    const centroid = labAt(lab, chosen)
    centroids.push(centroid)
    for (let cursor = 0; cursor < pixelCount; cursor += 1) {
      const distance = distanceSquared(lab, cursor, centroid)
      if (distance < (distances[cursor] ?? 0)) {
        distances[cursor] = distance
      }
    }
  }
  return centroids
}

const iterate = (lab: Float32Array, pixelCount: number, initial: readonly Lab[]): Clustering => {
  const assignments = new Int32Array(pixelCount)
  let current: readonly Lab[] = initial

  for (let iteration = 0; iteration < MAX_ITERATIONS; iteration += 1) {
    const sumL = new Float64Array(current.length)
    const sumA = new Float64Array(current.length)
    const sumB = new Float64Array(current.length)
    const count = new Int32Array(current.length)

    for (let cursor = 0; cursor < pixelCount; cursor += 1) {
      let best = 0
      let bestDistance = Number.POSITIVE_INFINITY
      for (let cluster = 0; cluster < current.length; cluster += 1) {
        const centroid = current[cluster]
        if (centroid === undefined) {
          continue
        }
        const distance = distanceSquared(lab, cursor, centroid)
        if (distance < bestDistance) {
          bestDistance = distance
          best = cluster
        }
      }
      assignments[cursor] = best
      sumL[best] = (sumL[best] ?? 0) + (lab[cursor * 3] ?? 0)
      sumA[best] = (sumA[best] ?? 0) + (lab[cursor * 3 + 1] ?? 0)
      sumB[best] = (sumB[best] ?? 0) + (lab[cursor * 3 + 2] ?? 0)
      count[best] = (count[best] ?? 0) + 1
    }

    let shift = 0
    const next = Arr.map(current, (centroid, cluster) => {
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
    if (shift < CONVERGENCE) {
      break
    }
  }

  return { centroids: current, assignments }
}

const cluster = (
  lab: Float32Array,
  pixelCount: number,
  previous: readonly Lab[] | null,
): Clustering => {
  if (previous !== null && previous.length === CLUSTERS) {
    return iterate(lab, pixelCount, previous)
  }
  const random = makeRandom(SEED)
  return iterate(lab, pixelCount, initCentroids(lab, pixelCount, random))
}

export { cluster }
export type { Clustering }
