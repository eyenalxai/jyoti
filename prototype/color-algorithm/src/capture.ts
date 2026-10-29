import { mkdir } from "node:fs/promises"
import path from "node:path"

interface Scene {
  readonly kind: "image" | "video"
  readonly file: string
  readonly seconds: number
  readonly start?: number
}

const root = path.resolve(import.meta.dir, "..")
const sceneRoot = path.join(root, "out", "scenes")
const captureRoot = path.join(root, "out", "frames")

const scenes: Record<string, Scene> = {
  document: { kind: "image", file: path.join(sceneRoot, "document.png"), seconds: 12 },
  editor: { kind: "image", file: path.join(sceneRoot, "editor.png"), seconds: 12 },
  movie: {
    kind: "video",
    file: "/home/ulezot/Downloads/kling_20260917_VIDEO_show_me_my_427_0.mp4",
    seconds: 26,
  },
  desktop: {
    kind: "video",
    file: "/home/ulezot/Videos/screenrecording-2026-09-21_14-42-08.mp4",
    seconds: 24,
    start: 12,
  },
  dynamic: { kind: "video", file: path.join(sceneRoot, "dynamic.mp4"), seconds: 24 },
  drift: { kind: "video", file: path.join(sceneRoot, "drift.mp4"), seconds: 30 },
  dark: { kind: "video", file: path.join(sceneRoot, "dark.mp4"), seconds: 20 },
  steps: { kind: "video", file: path.join(sceneRoot, "steps.mp4"), seconds: 32 },
}

const option = (name: string, fallback: string): string => {
  const index = process.argv.indexOf(`--${name}`)
  return index >= 0 ? (process.argv[index + 1] ?? fallback) : fallback
}

const sceneName = option("scene", "document")
const monitor = option("monitor", "HEADLESS-1")
const intervalMs = Number(option("interval", "666.6667"))
const scene = scenes[sceneName]
if (scene === undefined) {
  throw new Error(`Unknown scene ${sceneName}. Known: ${Object.keys(scenes).join(", ")}`)
}
const seconds = Number(option("seconds", String(scene.seconds)))

const output = path.join(captureRoot, sceneName)
await mkdir(output, { recursive: true })

const killHeadlessMpv = (): void => {
  const list = Bun.spawnSync(["pgrep", "-f", "mpv.*fs-screen-name=HEADLESS-1"])
  if (list.exitCode !== 0) {
    return
  }
  for (const line of list.stdout.toString().trim().split("\n")) {
    const pid = Number(line)
    if (Number.isInteger(pid) && pid > 0) {
      try {
        process.kill(pid, "SIGTERM")
      } catch {
        // Already gone.
      }
    }
  }
}

killHeadlessMpv()

const args = [
  "--no-audio",
  "--fs",
  `--fs-screen-name=${monitor}`,
  "--really-quiet",
  "--loop-file=inf",
  "--no-osc",
  "--no-input-default-bindings",
]
if (scene.kind === "image") {
  args.push("--image-display-duration=inf")
}
if (scene.start !== undefined) {
  args.push(`--start=${scene.start}`)
}
args.push(scene.file)

const player = Bun.spawn(["mpv", ...args], {
  stdout: "ignore",
  stderr: "ignore",
  stdin: "ignore",
})

await Bun.sleep(2_500)

const count = Math.max(1, Math.floor((seconds * 1000) / intervalMs))
const started = performance.now()
const gaps: number[] = []
let previous = started

for (let index = 0; index < count; index += 1) {
  const target = started + index * intervalMs
  const wait = target - performance.now()
  if (wait > 0) {
    await Bun.sleep(wait)
  }
  const file = path.join(output, `${index.toString().padStart(4, "0")}.png`)
  const shot = Bun.spawnSync([
    "grim",
    "-o",
    monitor,
    "-s",
    "0.0666667",
    "-t",
    "png",
    "-l",
    "1",
    file,
  ])
  if (shot.exitCode !== 0) {
    throw new Error(`grim failed: ${shot.stderr.toString()}`)
  }
  const now = performance.now()
  gaps.push(now - previous)
  previous = now
}

player.kill("SIGTERM")
await player.exited
killHeadlessMpv()

const actual = gaps.filter((gap) => gap > 0)
await Bun.write(
  path.join(output, "meta.json"),
  `${JSON.stringify(
    {
      scene: sceneName,
      file: scene.file,
      monitor,
      intervalMs,
      frames: count,
      measuredIntervalMs:
        actual.length > 0 ? actual.reduce((sum, gap) => sum + gap, 0) / actual.length : 0,
      maxIntervalMs: Math.max(0, ...actual),
      capturedAt: new Date().toISOString(),
    },
    null,
    2,
  )}\n`,
)

console.log(
  `${sceneName}: ${count} frames into ${path.relative(root, output)}, mean gap ${(
    actual.reduce((sum, gap) => sum + gap, 0) / Math.max(1, actual.length)
  ).toFixed(1)} ms`,
)
