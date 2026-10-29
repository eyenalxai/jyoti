import { mkdir } from "node:fs/promises"
import path from "node:path"

import type { AlgorithmId, Candidate, RunOptions } from "./algorithms"
import type { Lab } from "./color"
import type { PreparedFrame } from "./frame"
import type { RawTick, TickRecord } from "./metrics"

import { deltaEOk, hueDistance, parseHex, rgbToOklab } from "./color"
import { prepareFrame } from "./frame"
import { collectRaw, sequenceMetrics, simulateTemporal } from "./metrics"

const root = path.resolve(import.meta.dir, "..")
const frameRoot = path.join(root, "out", "frames")
const sceneNames = [
  "document",
  "editor",
  "movie",
  "desktop",
  "dynamic",
  "drift",
  "dark",
  "steps",
] as const
type SceneName = (typeof sceneNames)[number]

const baseOptions: RunOptions = {
  neutralThreshold: 0.03,
  neutralKelvin: 4000,
  chromaCap: true,
  k: 5,
  warmStart: false,
  outputLightness: 0.65,
  capBrightness: "fixed",
}

interface LoadedScene {
  readonly name: SceneName
  readonly meta: Record<string, unknown>
  readonly frames: readonly PreparedFrame[]
  readonly meanLabs: readonly Lab[]
  readonly p90: readonly number[]
}

const round = (value: number, digits = 4): number => Number(value.toFixed(digits))

const meanLab = (labs: readonly Lab[]): Lab => ({
  l: labs.reduce((sum, lab) => sum + lab.l, 0) / Math.max(1, labs.length),
  a: labs.reduce((sum, lab) => sum + lab.a, 0) / Math.max(1, labs.length),
  b: labs.reduce((sum, lab) => sum + lab.b, 0) / Math.max(1, labs.length),
})

const frameMeanLab = (frame: PreparedFrame): Lab => {
  let l = 0
  let a = 0
  let b = 0
  for (let index = 0; index < frame.pixelCount; index += 1) {
    l += frame.lab[index * 3] ?? 0
    a += frame.lab[index * 3 + 1] ?? 0
    b += frame.lab[index * 3 + 2] ?? 0
  }
  return { l: l / frame.pixelCount, a: a / frame.pixelCount, b: b / frame.pixelCount }
}

const readPng = (file: string): Uint8Array => {
  const result = Bun.spawnSync(["magick", file, "-depth", "8", "rgba:-"])
  if (result.exitCode !== 0) {
    throw new Error(`magick failed on ${file}: ${result.stderr.toString()}`)
  }
  return new Uint8Array(result.stdout)
}

const loadScene = async (name: SceneName): Promise<LoadedScene> => {
  const directory = path.join(frameRoot, name)
  const meta = (await Bun.file(path.join(directory, "meta.json")).json()) as Record<string, unknown>
  const count = Number(meta.frames)
  const frames: PreparedFrame[] = []
  for (let index = 0; index < count; index += 1) {
    const file = path.join(directory, `${index.toString().padStart(4, "0")}.png`)
    const data = readPng(file)
    frames.push(prepareFrame({ width: 128, height: 72, data }))
  }
  return {
    name,
    meta,
    frames,
    meanLabs: frames.map(frameMeanLab),
    p90: frames.map((frame) => frame.p90Chroma),
  }
}

const compact = (candidate: Candidate): Record<string, unknown> => ({
  hex: candidate.hex,
  l: round(candidate.lch.l),
  c: round(candidate.lch.c),
  h: round(candidate.lch.h, 1),
  n: candidate.neutral,
  k: candidate.kelvin,
  capped: candidate.capped,
  gamut: candidate.gamutClamped,
  meta: candidate.meta,
})

const compactTick = (record: TickRecord): Record<string, unknown> => ({
  t: Math.round(record.timeMs),
  costMs: round(record.costMs, 3),
  p90: round(record.raw.meta.p90Chroma as number, 4),
  raw: compact(record.raw),
  smooth: compact(record.smooth),
  published: compact(record.published),
})

const settleAfterLabs = (
  labs: readonly Lab[],
  start: number,
  final: Lab,
  limit = labs.length - 1,
): number | null => {
  for (let index = start + 1; index <= limit; index += 1) {
    const current = labs[index] as Lab
    const next = labs[index + 1]
    if (
      deltaEOk(current, final) < 0.05 &&
      (next === undefined || index + 1 > limit || deltaEOk(next, final) < 0.05)
    ) {
      return index - start
    }
  }
  return null
}

const findBiggestCut = (meanLabs: readonly Lab[]): { index: number; delta: number } => {
  let best = { index: 0, delta: 0 }
  for (let index = 1; index < meanLabs.length; index += 1) {
    const delta = deltaEOk(meanLabs[index - 1] as Lab, meanLabs[index] as Lab)
    if (delta > best.delta) {
      best = { index, delta }
    }
  }
  return best
}

const stepColors = [
  { name: "white", hex: "#F5F5F0" },
  { name: "red", hex: "#C0392B" },
  { name: "blue", hex: "#2980B9" },
  { name: "green", hex: "#27AE60" },
  { name: "black", hex: "#101018" },
] as const

const classify = (lab: Lab): string => {
  let best = stepColors[0].name
  let bestDelta = Number.POSITIVE_INFINITY
  for (const color of stepColors) {
    const delta = deltaEOk(lab, rgbToOklab(parseHex(color.hex)))
    if (delta < bestDelta) {
      bestDelta = delta
      best = color.name
    }
  }
  return best
}

interface StepSegment {
  readonly name: string
  readonly start: number
  readonly end: number
  readonly expected: Lab
}

const stepSegments = (meanLabs: readonly Lab[]): readonly StepSegment[] => {
  const labels = meanLabs.map(classify)
  const segments: StepSegment[] = []
  let start = 0
  for (let index = 1; index <= labels.length; index += 1) {
    if (index === labels.length || labels[index] !== labels[start]) {
      const name = labels[start] as string
      const color = stepColors.find((entry) => entry.name === name) ?? stepColors[0]
      segments.push({
        name,
        start,
        end: index - 1,
        expected: rgbToOklab(parseHex(color.hex)),
      })
      start = index
    }
  }
  return segments.filter((segment) => segment.end - segment.start >= 2)
}

const algorithmIds: readonly AlgorithmId[] = ["dominant", "saliency", "histogram"]

interface SceneResult {
  readonly scene: SceneName
  readonly meta: Record<string, unknown>
  readonly p90: readonly number[]
  readonly meanLabs: readonly Lab[]
  readonly algorithms: Record<string, Record<string, unknown>>
  readonly ticks: Record<string, readonly Record<string, unknown>[]>
}

const scenes: LoadedScene[] = []
for (const name of sceneNames) {
  scenes.push(await loadScene(name))
}

const results: SceneResult[] = []
const temporalSweep: Record<string, Record<string, unknown>[]> = {}
const deadbandSweep: Record<string, Record<string, unknown>[]> = {}
const capSweep: Record<string, Record<string, unknown>[]> = {}
const neutralSweep: Record<string, Record<string, unknown>> = {}

for (const scene of scenes) {
  const algorithms: Record<string, Record<string, unknown>> = {}
  const ticks: Record<string, readonly Record<string, unknown>[]> = {}
  const rawByAlgorithm = new Map<string, readonly RawTick[]>()

  for (const algorithm of algorithmIds) {
    const raws = collectRaw(algorithm, scene.frames, baseOptions)
    rawByAlgorithm.set(algorithm, raws)
    const records = simulateTemporal(raws, 450, 0.02)
    const metrics = sequenceMetrics(algorithm, records, 0.02)
    const hueErrors = records
      .map((record, index) =>
        record.raw.neutral
          ? null
          : hueDistance(record.raw.lch.h, scene.frames[index]?.meanHue ?? 0),
      )
      .filter((value): value is number => value !== null)
      .sort((a, b) => a - b)
    algorithms[algorithm] = {
      ...metrics,
      hueErrorMean: round(
        hueErrors.reduce((sum, value) => sum + value, 0) / Math.max(1, hueErrors.length),
        2,
      ),
      hueErrorP90: round(hueErrors[Math.floor(hueErrors.length * 0.9)] ?? 0, 2),
      hueErrorMax: round(hueErrors[hueErrors.length - 1] ?? 0, 2),
      referenceHueWeight: round(
        scene.frames.reduce((sum, frame) => sum + frame.hueWeight, 0) /
          Math.max(1, scene.frames.length),
        1,
      ),
      output: records.map((record) => record.published.hex),
      meanRawHex: records[Math.floor(records.length / 2)]?.raw.hex ?? null,
      p90ChromaMean: round(
        scene.p90.reduce((sum, value) => sum + value, 0) / Math.max(1, scene.p90.length),
      ),
      p90ChromaMin: round(Math.min(...scene.p90)),
      p90ChromaMax: round(Math.max(...scene.p90)),
    }
    ticks[algorithm] = records.map(compactTick)
  }

  const warm = collectRaw("dominant", scene.frames, { ...baseOptions, warmStart: true })
  const warmRecords = simulateTemporal(warm, 450, 0.02)
  const warmHueErrors = warmRecords
    .map((record, index) =>
      record.raw.neutral ? null : hueDistance(record.raw.lch.h, scene.frames[index]?.meanHue ?? 0),
    )
    .filter((value): value is number => value !== null)
    .sort((a, b) => a - b)
  algorithms["dominant-warm"] = {
    ...sequenceMetrics("dominant", warmRecords, 0.02),
    hueErrorMean: round(
      warmHueErrors.reduce((sum, value) => sum + value, 0) / Math.max(1, warmHueErrors.length),
      2,
    ),
    hueErrorP90: round(warmHueErrors[Math.floor(warmHueErrors.length * 0.9)] ?? 0, 2),
    hueErrorMax: round(warmHueErrors[warmHueErrors.length - 1] ?? 0, 2),
    output: warmRecords.map((record) => record.published.hex),
    p90ChromaMean: round(
      scene.p90.reduce((sum, value) => sum + value, 0) / Math.max(1, scene.p90.length),
    ),
  }
  ticks["dominant-warm"] = warmRecords.map(compactTick)

  if (scene.name === "movie") {
    const cut = findBiggestCut(scene.meanLabs)
    for (const key of [...algorithmIds, "dominant-warm"]) {
      const algorithmTicks = ticks[key] as readonly Record<string, unknown>[]
      const publishedLabs = algorithmTicks.map((tick) =>
        compactLab(tick.published as Record<string, unknown>),
      )
      const last = meanLab(publishedLabs.slice(-6))
      const settle = settleAfterLabs(publishedLabs, cut.index, last)
      const metrics = algorithms[key] as Record<string, unknown>
      metrics.cutIndex = cut.index
      metrics.cutDelta = round(cut.delta)
      metrics.settleTicks = settle
    }
  }

  if (scene.name === "steps") {
    const segments = stepSegments(scene.meanLabs)
    for (const key of [...algorithmIds, "dominant-warm"]) {
      const algorithmTicks = ticks[key] as readonly Record<string, unknown>[]
      const publishedLabs = algorithmTicks.map((tick) =>
        compactLab(tick.published as Record<string, unknown>),
      )
      const settles: number[] = []
      const steady: number[] = []
      for (let index = 0; index < segments.length - 1; index += 1) {
        const segment = segments[index] as StepSegment
        const tail = publishedLabs.slice(Math.max(segment.start, segment.end - 2), segment.end + 1)
        const steadyLab = meanLab(tail)
        const settle = settleAfterLabs(publishedLabs, segment.start, steadyLab, segment.end)
        if (settle !== null) {
          settles.push(settle)
        }
        steady.push(deltaEOk(steadyLab, segment.expected))
      }
      const metrics = algorithms[key] as Record<string, unknown>
      metrics.segments = segments.map((segment) => ({
        name: segment.name,
        start: segment.start,
        end: segment.end,
        expected: segment.expected,
      }))
      metrics.meanSettleTicks =
        settles.length > 0
          ? round(settles.reduce((sum, value) => sum + value, 0) / settles.length, 2)
          : null
      metrics.meanSteadyDelta =
        steady.length > 0
          ? round(steady.reduce((sum, value) => sum + value, 0) / steady.length)
          : null
      metrics.maxSettleTicks = settles.length > 0 ? Math.max(...settles) : null
    }
  }

  results.push({
    scene: scene.name,
    meta: scene.meta,
    p90: scene.p90.map((value) => round(value)),
    meanLabs: scene.meanLabs.map((lab) => ({ l: round(lab.l), a: round(lab.a), b: round(lab.b) })),
    algorithms,
    ticks,
  })

  for (const algorithm of algorithmIds) {
    const raws = rawByAlgorithm.get(algorithm) as readonly RawTick[]
    for (const tau of [0, 300, 450, 700, 900]) {
      const key = `${algorithm}|tau=${tau}`
      const records = simulateTemporal(raws, Math.max(1, tau), 0.02)
      const metrics = sequenceMetrics(algorithm, records, 0.02)
      const row = temporalSweep[key] ?? []
      row.push({
        scene: scene.name,
        tau,
        jitter: round(metrics.smoothJitter),
        publishRate: round(metrics.publishRate, 3),
        hueFlips: metrics.hueFlips,
      })
      temporalSweep[key] = row
    }
    for (const deadband of [0.01, 0.015, 0.02, 0.03]) {
      const key = `${algorithm}|deadband=${deadband}`
      const records = simulateTemporal(raws, 450, deadband)
      const metrics = sequenceMetrics(algorithm, records, deadband)
      const row = deadbandSweep[key] ?? []
      row.push({
        scene: scene.name,
        deadband,
        publishRate: round(metrics.publishRate, 3),
        publishedJitter: round(metrics.publishedJitter),
        changes: metrics.publishedChanges,
      })
      deadbandSweep[key] = row
    }
  }

  for (const threshold of [0.02, 0.025, 0.03, 0.035, 0.04, 0.05]) {
    const neutral = scene.p90.filter((value) => value < threshold).length
    const key = `threshold=${threshold}`
    const row = neutralSweep[key] ?? {}
    row[scene.name] = round(neutral / Math.max(1, scene.p90.length), 3)
    neutralSweep[key] = row
  }
}

for (const sceneName of ["dynamic", "steps"] as const) {
  const scene = scenes.find((entry) => entry.name === sceneName) as LoadedScene
  for (const algorithm of algorithmIds) {
    for (const chromaCap of [false, true]) {
      const raws = collectRaw(algorithm, scene.frames, { ...baseOptions, chromaCap })
      const records = simulateTemporal(raws, 450, 0.02)
      const metrics = sequenceMetrics(algorithm, records, 0.02)
      const key = `${algorithm}|cap=${chromaCap}`
      const row = capSweep[key] ?? []
      row.push({
        scene: scene.name,
        cap: chromaCap,
        meanC: round(metrics.meanC),
        meanL: round(metrics.meanL),
        gamutClamps: records.filter((record) => record.raw.gamutClamped).length,
        capped: records.filter((record) => record.raw.capped).length,
      })
      capSweep[key] = row
    }
  }
}

function compactLab(value: Record<string, unknown>): Lab {
  return {
    l: Number(value.l),
    a: Number(value.c) * Math.cos((Number(value.h) * Math.PI) / 180),
    b: Number(value.c) * Math.sin((Number(value.h) * Math.PI) / 180),
  }
}

const summary = {
  generatedAt: new Date().toISOString(),
  baseOptions,
  scenes: results,
  neutralSweep,
  temporalSweep,
  deadbandSweep,
  capSweep,
}

await mkdir(path.join(root, "out"), { recursive: true })
await Bun.write(path.join(root, "out", "report.json"), `${JSON.stringify(summary, null, 2)}\n`)

for (const result of results) {
  const line = (["dominant", "saliency", "histogram", "dominant-warm"] as const)
    .map((key) => {
      const metrics = result.algorithms[key] as Record<string, unknown>
      return `${key}: n=${Number(metrics.neutralRate).toFixed(2)} j=${Number(metrics.rawJitter).toFixed(3)} pub=${Number(metrics.publishRate).toFixed(2)} flips=${metrics.hueFlips} cost=${Number(metrics.meanCostMs).toFixed(2)}ms`
    })
    .join(" | ")
  console.log(`${result.scene}: ${line}`)
}

const p90Line = results
  .map((result) => {
    const values = result.p90 as readonly number[]
    return `${result.scene}=${(values.reduce((sum, value) => sum + value, 0) / Math.max(1, values.length)).toFixed(3)}`
  })
  .join(" ")
console.log(`mean p90 chroma: ${p90Line}`)
console.log(`wrote ${path.join(root, "out", "report.json")}`)
