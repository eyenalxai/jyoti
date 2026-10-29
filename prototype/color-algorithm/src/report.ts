import path from "node:path"

const root = path.resolve(import.meta.dir, "..")
const report = (await Bun.file(path.join(root, "out", "report.json")).json()) as Report

interface Metric {
  readonly neutralRate: number
  readonly rawJitter: number
  readonly smoothJitter: number
  readonly publishedJitter: number
  readonly publishRate: number
  readonly neutralFlips: number
  readonly hueFlips: number
  readonly meanL: number
  readonly meanC: number
  readonly meanCostMs: number
  readonly maxCostMs: number
  readonly hueErrorMean?: number
  readonly hueErrorP90?: number
  readonly output?: readonly string[]
}

interface Scene {
  readonly scene: string
  readonly p90: readonly number[]
  readonly algorithms: Record<string, Metric>
  readonly ticks: Record<string, readonly Tick[]>
}

interface Tick {
  readonly t: number
  readonly p90: number
  readonly raw: {
    readonly hex: string
    readonly l: number
    readonly c: number
    readonly h: number
    readonly n: boolean
    readonly k: number | null
    readonly meta: Record<string, number>
  }
  readonly published: {
    readonly hex: string
    readonly l: number
    readonly c: number
    readonly h: number
    readonly n: boolean
    readonly k: number | null
  }
}

interface Report {
  readonly generatedAt: string
  readonly scenes: readonly Scene[]
  readonly neutralSweep: Record<string, Record<string, number>>
  readonly temporalSweep: Record<string, readonly Record<string, number | string>[]>
  readonly deadbandSweep: Record<string, readonly Record<string, number | string>[]>
  readonly capSweep: Record<string, readonly Record<string, number | string>[]>
}

const sceneByName = (name: string): Scene => {
  const scene = report.scenes.find((entry) => entry.scene === name)
  if (scene === undefined) {
    throw new Error(`missing scene ${name}`)
  }
  return scene
}

const frameData = async (scene: string, index: number): Promise<string> => {
  const file = path.join(root, "out", "frames", scene, `${index.toString().padStart(4, "0")}.png`)
  const bytes = await Bun.file(file).arrayBuffer()
  return `data:image/png;base64,${Buffer.from(bytes).toString("base64")}`
}

const pct = (value: number): string => `${Math.round(value * 100)}%`
const num = (value: number, digits = 2): string => value.toFixed(digits)
const deg = (value: number): string => `${Math.round(value)}°`

const swatch = (hex: string, label: string, neutral = false): string =>
  `<span class="swatch-wrap"><span class="swatch${neutral ? " neutral" : ""}" style="background:${hex}"></span><span class="swatch-label">${label}</span></span>`

const algoName: Record<string, string> = {
  dominant: "Dominant (cold)",
  "dominant-warm": "Dominant (warm start)",
  saliency: "Saliency background",
  histogram: "Hue histogram",
}

const sceneTitle: Record<string, string> = {
  document: "White document",
  editor: "Dark editor",
  movie: "Movie: beach → fire",
  desktop: "Dark desktop UI",
  dynamic: "Fast hue drift",
  drift: "Slow hue drift",
  dark: "Dim scene",
  steps: "Solid-colour steps",
}

const sceneNote: Record<string, string> = {
  document:
    "A white page with black text, a blue logo and blue links. The whole frame must read neutral. p90 chroma is 0.003, far below the 0.03 gate.",
  editor:
    "A dark code editor with muted syntax colours and a blue status bar. p90 chroma is 0.025: still neutral at 0.03, colourful at 0.025. This is the closest call in the set.",
  movie:
    "A real five-second clip that crossfades from a blue-grey beach to a close-up of a fire. The beach should read blue, the fire orange. This is the product-feel test.",
  desktop:
    "A real screen recording of a dark T3 Code window with a small webcam thumbnail. The frame is mostly dark UI; p90 chroma is 0.008.",
  dynamic:
    "A saturated test pattern rotating its hue a full turn every seven seconds. Every tick is a large change; it tests whether the algorithm follows rather than how still it is.",
  drift:
    "A large coloured field drifting through the hue wheel over 30 seconds, beside a static grey bar and a black letterbox. Dominant and histogram track it; saliency locks onto the grey bar.",
  dark: "The same test pattern at -0.42 brightness and 0.45 saturation. A dim, colourful scene should produce a dim coloured light, not a bright tint.",
  steps:
    "Off-white, red, blue, green and near-black, three seconds each, looping. This is the ground-truth test for hard cuts; the white and black segments must hit the neutral fallback.",
}

const sampleIndices: Record<string, readonly number[]> = {
  document: [0],
  editor: [0],
  movie: [0, 4, 10, 18, 26, 34],
  desktop: [0],
  dynamic: [0, 8, 16, 24, 32],
  drift: [0, 8, 16, 24, 32, 40],
  dark: [0, 8, 16, 24],
  steps: [0, 1, 6, 11, 15, 20, 24, 29, 34, 42],
}

const timeline = (scene: Scene, algorithm: string): string => {
  const ticks = scene.ticks[algorithm] ?? []
  const cells = ticks
    .map((tick) => {
      const value = tick.published
      const color = value.n ? "#ffcea6" : value.hex
      const label = value.n ? `neutral ${value.k ?? 4000} K` : value.hex
      return `<span class="tick${value.n ? " tick-neutral" : ""}" style="background:${color}" title="t=${Math.round(tick.t)} ms · ${label}"></span>`
    })
    .join("")
  return `<div class="timeline-row"><span class="timeline-name">${algoName[algorithm] ?? algorithm}</span><span class="ticks">${cells}</span></div>`
}

const sceneSection = async (name: string): Promise<string> => {
  const scene = sceneByName(name)
  const indices = sampleIndices[name] ?? [0]
  const samples = await Promise.all(
    indices.map(async (index) => {
      const image = await frameData(name, index)
      const cells = (["dominant-warm", "saliency", "histogram"] as const)
        .map((algorithm) => {
          const tick = scene.ticks[algorithm]?.[index]
          if (tick === undefined) {
            return ""
          }
          return swatch(
            tick.raw.hex,
            tick.raw.n ? `${tick.raw.k ?? 4000} K` : tick.raw.hex,
            tick.raw.n,
          )
        })
        .join("")
      return `<figure class="sample"><img src="${image}" alt="${name} frame ${index}" /><figcaption><span class="frame-index">${index.toString().padStart(2, "0")}</span><span class="sample-swatches">${cells}</span></figcaption></figure>`
    }),
  )

  const showTimeline = ["movie", "dynamic", "drift", "dark", "steps"].includes(name)
  return `<section class="scene">
    <div class="scene-head"><h2>${sceneTitle[name] ?? name}</h2><span class="kicker">${scene.p90.length} frames</span></div>
    <p class="scene-note">${sceneNote[name] ?? ""}</p>
    <div class="samples">${samples.join("")}</div>
    ${showTimeline ? `<div class="timelines">${(["dominant-warm", "saliency", "histogram"] as const).map((algorithm) => timeline(scene, algorithm)).join("")}</div>` : ""}
  </section>`
}

const metricCell = (
  metric: Metric | undefined,
  key: "hueErrorMean" | "rawJitter" | "neutralRate" | "meanCostMs",
): string => {
  if (metric === undefined) {
    return "—"
  }
  if (key === "hueErrorMean") {
    return metric.hueErrorMean === undefined ? "—" : deg(metric.hueErrorMean)
  }
  if (key === "rawJitter") {
    return num(metric.rawJitter, 3)
  }
  if (key === "neutralRate") {
    return pct(metric.neutralRate)
  }
  return `${num(metric.meanCostMs)} ms`
}

const summaryRows = ["document", "editor", "desktop", "movie", "dynamic", "drift", "dark", "steps"]
  .map((name) => {
    const scene = sceneByName(name)
    const algorithms = ["dominant-warm", "saliency", "histogram"] as const
    return `<tr><th>${sceneTitle[name] ?? name}</th>${algorithms
      .map((algorithm) => {
        const metric = scene.algorithms[algorithm]
        const tick = scene.ticks[algorithm]?.[0]
        return `<td>${tick === undefined ? "—" : swatch(tick.raw.hex, tick.raw.n ? "neutral" : tick.raw.hex, tick.raw.n)}<span class="cell-metric">${metricCell(metric, "hueErrorMean")} · ${metricCell(metric, "rawJitter")}</span></td>`
      })
      .join("")}</tr>`
  })
  .join("")

const algorithmRows = (["dominant", "dominant-warm", "saliency", "histogram"] as const)
  .map((algorithm) => {
    const names = ["document", "editor", "desktop"]
    const neutral = names.map((name) => sceneByName(name).algorithms[algorithm]?.neutralRate ?? 0)
    const meanNeutral = neutral.reduce((sum, value) => sum + value, 0) / neutral.length
    const movie = sceneByName("movie").algorithms[algorithm]
    const drift = sceneByName("drift").algorithms[algorithm]
    const dark = sceneByName("dark").algorithms[algorithm]
    return `<tr><th>${algoName[algorithm]}</th><td>${pct(meanNeutral)}</td><td>${movie === undefined ? "—" : deg(movie.hueErrorMean ?? 0)}</td><td>${drift === undefined ? "—" : deg(drift.hueErrorMean ?? 0)}</td><td>${dark === undefined ? "—" : num(dark.rawJitter, 3)}</td><td>${dark === undefined ? "—" : num(dark.meanCostMs)} ms</td></tr>`
  })
  .join("")

const neutralRows = Object.entries(report.neutralSweep)
  .map(([threshold, values]) => {
    return `<tr><th>${threshold.replace("threshold=", "")}</th>${["document", "editor", "desktop", "movie", "dynamic", "drift", "dark"].map((name) => `<td>${pct(values[name] ?? 0)}</td>`).join("")}</tr>`
  })
  .join("")

const deadbandRows = Object.entries(report.deadbandSweep)
  .filter(([key]) => key.startsWith("dominant|"))
  .map(([key, rows]) => {
    const band = key.split("=")[1]
    const drift = rows.find((row) => row.scene === "drift")
    const steps = rows.find((row) => row.scene === "steps")
    return `<tr><th>${band}</th><td>${drift === undefined ? "—" : pct(Number(drift.publishRate))}</td><td>${steps === undefined ? "—" : pct(Number(steps.publishRate))}</td></tr>`
  })
  .join("")

const capRows = Object.entries(report.capSweep)
  .filter(([key]) => key.startsWith("dominant|"))
  .map(([key, rows]) => {
    const dynamic = rows.find((row) => row.scene === "dynamic")
    const steps = rows.find((row) => row.scene === "steps")
    return `<tr><th>${key.includes("true") ? "Capped at 0.13" : "No cap"}</th><td>${dynamic === undefined ? "—" : num(Number(dynamic.meanC), 3)}</td><td>${steps === undefined ? "—" : num(Number(steps.meanC), 3)}</td><td>${dynamic === undefined ? "—" : dynamic.gamutClamps}</td></tr>`
  })
  .join("")

const sections = await Promise.all(
  ["document", "editor", "movie", "desktop", "drift", "dynamic", "dark", "steps"].map(sceneSection),
)

const html = `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>JYO-23 · Choose the color algorithm</title>
<style>
  :root { --ink:#201f1c; --muted:#6b675f; --paper:#faf8f3; --card:#fffdf8; --line:#dfdad0; --accent:#b4552d; --blue:#2d5f8b; }
  * { box-sizing:border-box; }
  body { margin:0; background:var(--paper); color:var(--ink); font-family:Inter,ui-sans-serif,system-ui,-apple-system,"Segoe UI",sans-serif; line-height:1.55; }
  main { max-width:1180px; margin:0 auto; padding:56px 40px 96px; }
  .kicker { font-size:11px; letter-spacing:.16em; text-transform:uppercase; color:var(--muted); font-weight:650; }
  h1 { font-size:52px; line-height:1.02; letter-spacing:-.03em; margin:10px 0 18px; max-width:900px; }
  .lede { font-size:20px; color:#3f3c36; max-width:820px; margin:0 0 36px; }
  h2 { font-size:24px; letter-spacing:-.01em; margin:0; }
  h3 { font-size:14px; letter-spacing:.08em; text-transform:uppercase; color:var(--muted); margin:28px 0 10px; }
  a { color:var(--blue); }
  .stats { display:grid; grid-template-columns:repeat(4,1fr); gap:12px; margin:28px 0 44px; }
  .stat { background:var(--card); border:1px solid var(--line); border-radius:12px; padding:16px 18px; }
  .stat b { display:block; font-size:26px; letter-spacing:-.02em; }
  .stat span { font-size:12px; color:var(--muted); }
  .verdict { background:#15231c; color:#edf4ee; border-radius:16px; padding:28px 32px; margin:34px 0 44px; }
  .verdict .kicker { color:#9fbea8; }
  .verdict h2 { font-size:30px; margin:8px 0 10px; }
  .verdict p { color:#c8d8cc; max-width:900px; margin:0 0 14px; }
  .verdict code { background:#25382d; color:#e5f0e7; padding:2px 6px; border-radius:5px; }
  .verdict .grid { display:grid; grid-template-columns:repeat(3,1fr); gap:14px; margin-top:18px; }
  .verdict .grid div { background:#1d2e24; border:1px solid #2d4436; border-radius:10px; padding:12px 14px; font-size:13px; color:#c8d8cc; }
  .verdict .grid b { display:block; color:#f2f7f3; font-size:14px; margin-bottom:3px; }
  table { width:100%; border-collapse:collapse; background:var(--card); border:1px solid var(--line); border-radius:12px; overflow:hidden; font-size:13px; }
  th, td { text-align:left; padding:11px 13px; border-bottom:1px solid var(--line); vertical-align:middle; }
  thead th { background:#f0ece3; font-size:11px; letter-spacing:.08em; text-transform:uppercase; color:var(--muted); }
  tbody tr:last-child th, tbody tr:last-child td { border-bottom:0; }
  tbody th { font-weight:600; }
  .swatch-wrap { display:inline-flex; align-items:center; gap:7px; margin-right:10px; white-space:nowrap; }
  .swatch { width:22px; height:22px; border-radius:6px; border:1px solid rgba(0,0,0,.14); display:inline-block; box-shadow:inset 0 0 0 1px rgba(255,255,255,.18); }
  .swatch.neutral { border-style:dashed; }
  .swatch-label { font-family:ui-monospace,SFMono-Regular,Menlo,monospace; font-size:11px; color:var(--muted); }
  .cell-metric { display:block; margin-top:4px; color:var(--muted); font-size:11px; }
  .scene { margin:52px 0 0; }
  .scene-head { display:flex; align-items:baseline; justify-content:space-between; gap:16px; border-top:1px solid var(--line); padding-top:26px; }
  .scene-note { color:#4d4a43; max-width:900px; margin:8px 0 18px; }
  .samples { display:grid; grid-template-columns:repeat(auto-fit,minmax(150px,1fr)); gap:12px; }
  .sample { margin:0; background:var(--card); border:1px solid var(--line); border-radius:10px; overflow:hidden; }
  .sample img { display:block; width:100%; image-rendering:auto; }
  .sample figcaption { padding:8px 9px 10px; }
  .frame-index { font-family:ui-monospace,SFMono-Regular,Menlo,monospace; font-size:10px; color:#9a948a; display:block; margin-bottom:5px; }
  .sample-swatches { display:flex; gap:5px; }
  .sample-swatches .swatch { width:20px; height:20px; border-radius:5px; }
  .sample-swatches .swatch-label { display:none; }
  .timelines { margin-top:16px; background:var(--card); border:1px solid var(--line); border-radius:10px; padding:12px 14px; }
  .timeline-row { display:flex; align-items:center; gap:12px; margin:6px 0; }
  .timeline-name { width:150px; font-size:11px; color:var(--muted); flex:none; }
  .ticks { display:flex; gap:2px; flex-wrap:nowrap; overflow:hidden; }
  .tick { width:13px; height:20px; border-radius:3px; display:inline-block; border:1px solid rgba(0,0,0,.12); }
  .tick-neutral { border-style:dashed; }
  .cols { display:grid; grid-template-columns:1fr 1fr; gap:28px; }
  .note { background:#f2ede3; border-left:3px solid var(--accent); padding:14px 18px; border-radius:0 8px 8px 0; color:#4b463d; font-size:14px; }
  footer { margin-top:64px; padding-top:20px; border-top:1px solid var(--line); color:var(--muted); font-size:12px; }
  @media (max-width:800px) { main { padding:32px 18px 64px; } h1 { font-size:36px; } .stats, .cols, .verdict .grid { grid-template-columns:1fr; } }
</style>
</head>
<body>
<main>
  <p class="kicker">JYO-23 · wayfinder:prototype · 2026-10-29</p>
  <h1>Choose the color algorithm</h1>
  <p class="lede">Three candidates, eight real captured scenes, 1.5 Hz. The question is not which algorithm is cleverest. It is which one makes a dark room feel like the screen without turning a white document blue.</p>

  <div class="stats">
    <div class="stat"><b>8</b><span>real scenes, 128 × 72</span></div>
    <div class="stat"><b>${report.scenes.reduce((sum, scene) => sum + scene.p90.length, 0)}</b><span>frames at 1.5 Hz</span></div>
    <div class="stat"><b>3</b><span>candidate algorithms</span></div>
    <div class="stat"><b>${num(Math.max(...report.scenes.flatMap((scene) => Object.values(scene.algorithms).map((metric) => metric.maxCostMs))), 1)} ms</b><span>worst frame, TS only</span></div>
  </div>

  <section class="verdict">
    <p class="kicker">Recommendation</p>
    <h2>Chroma-weighted OKLab dominant, warm-started, with a neutral gate</h2>
    <p>The research default survives the prototype. k-means on the 128 × 72 OKLab grid, k = 5, score <code>population^0.8 × (0.2 + 0.8 × min(C / 0.15, 1))</code>, drop clusters under 3% population, keep the winner's hue. Start each tick from the previous tick's centroids: it removes the single-tick cluster flips without slowing the hard cuts.</p>
    <p>The derived color is emitted at a fixed OKLab lightness of <code>0.65</code>, with chroma capped at <code>0.13</code> (the research cap at full brightness). The scene's mean OKLab lightness is published separately as a brightness signal; the UX ticket owns the mapping to bulb brightness. A neutral scene emits the temperature-mode fallback at <code>4000 K</code>, not an RGB white.</p>
    <div class="grid">
      <div><b>Neutral gate</b>p90 OKLCH chroma &lt; 0.03, with hysteresis to 0.045 before leaving neutral.</div>
      <div><b>Deadband</b>ΔE<sub>OK</sub> 0.02 against the last acknowledged color, inherited from JYO-21.</div>
      <div><b>Smoothing</b>450 ms OKLab EMA for color, 900 ms for the brightness signal.</div>
    </div>
  </section>

  <h3>What the scenes show</h3>
  <table>
    <thead><tr><th>Scene</th><th>Dominant, warm</th><th>Saliency background</th><th>Hue histogram</th></tr></thead>
    <tbody>${summaryRows}</tbody>
  </table>
  <p class="note">Cell colours are the first published tick; the small line under each is mean hue error against the frame's chroma-weighted mean hue, then raw ΔE jitter per tick. Hue error is only meaningful on single-hue scenes; the movie and the two drift scenes carry the weight.</p>

  <h3>Headline checks</h3>
  <table>
    <thead><tr><th>Algorithm</th><th>Neutral on document/editor/desktop</th><th>Movie hue error</th><th>Drift hue error</th><th>Dim-scene jitter</th><th>Mean cost</th></tr></thead>
    <tbody>${algorithmRows}</tbody>
  </table>

  <h3>Neutral threshold sweep</h3>
  <table>
    <thead><tr><th>p90 chroma gate</th><th>Document</th><th>Editor</th><th>Desktop</th><th>Movie</th><th>Fast drift</th><th>Slow drift</th><th>Dim</th></tr></thead>
    <tbody>${neutralRows}</tbody>
  </table>
  <p class="note">0.03 is the only gate that makes the editor neutral without touching the colourful scenes. 0.025 would tint a dark code editor; 0.04 changes nothing else. The document is neutral at every threshold tested.</p>

  <div class="cols">
    <div>
      <h3>Deadband sweep</h3>
      <table>
        <thead><tr><th>ΔE<sub>OK</sub></th><th>Slow-drift publishes</th><th>Step publishes</th></tr></thead>
        <tbody>${deadbandRows}</tbody>
      </table>
    </div>
    <div>
      <h3>Chroma cap</h3>
      <table>
        <thead><tr><th>Mode</th><th>Fast-drift mean C</th><th>Steps mean C</th><th>Gamut clamps</th></tr></thead>
        <tbody>${capRows}</tbody>
      </table>
    </div>
  </div>

  ${sections.join("")}

  <h3>What failed</h3>
  <p class="note"><strong>Saliency background is out.</strong> On the slow-drift scene it locks onto the static grey bar and reports chroma 0.00 for the whole 30 seconds while the coloured field sweeps the hue wheel; on the movie it averages the dark background and reports a mean hue error of 41°. It is stable and cheap, and it is exactly the “just picks the background” failure the research warned about.</p>
  <p class="note"><strong>Hue histogram is viable but second.</strong> It tracks the slow drift as well as dominant and is faster (0.1 ms vs 0.3–1.3 ms), but its 10° bins raise the movie's p90 hue error from 13° to 21°, and it can sit on one bin through a dim scene. Good fallback if k-means ever needs replacing; not the default.</p>
  <p class="note"><strong>Cold-started k-means flips.</strong> Without warm start, the dominant algorithm produces single-tick hue jumps on the dim scene (15 hue flips) and occasional flips on the fast-drift pattern. Warm start drops the dim scene to zero at no extra cost, and both variants follow every hard cut in the steps scene within one tick.</p>

  <footer>Prototype branch <code>prototype/color-algorithm</code>. Captures are grim frames of a headless 1920 × 1080 output, 1.5 Hz, downscaled to 128 × 72 before analysis. Raw frames and the JSON are in <code>prototype/color-algorithm/out</code>.</footer>
</main>
</body>
</html>`

await Bun.write(path.join(root, "out", "report.html"), html)
console.log(
  `wrote ${path.join(root, "out", "report.html")} (${(html.length / 1024).toFixed(0)} KB)`,
)
