import { RGBA } from "@opentui/core"
import { Array as Arr } from "effect"
import { createMemo } from "solid-js"

import type { CaptureHealth, SyncPhase, SyncStatus } from "@/lib/screen/sync/status"

import { truncate } from "@/lib/format"
import { restoreSummary, syncKeys } from "@/lib/screen/sync/copy"
import { palette } from "@/lib/theme"

interface SyncStripProps {
  readonly status: SyncStatus
  readonly width: number
}

const healthWord = (capture: CaptureHealth): string => {
  if (capture === "black") {
    return "black frame"
  }
  if (capture === "stale") {
    return "capture stale"
  }
  if (capture === "lost") {
    return "capture lost"
  }
  if (capture === "switching") {
    return "switching monitor"
  }
  return ""
}

interface PhaseView {
  readonly word: string
  readonly glyph: string
  readonly tone: RGBA
}

const phaseView = (phase: SyncPhase, capture: CaptureHealth, flashing: boolean): PhaseView => {
  if (phase === "starting" || phase === "stopping") {
    return { word: phase, glyph: "◌", tone: palette.muted }
  }
  if (phase === "paused") {
    const health = healthWord(capture)
    return {
      word: health.length === 0 ? "paused" : `paused · ${health}`,
      glyph: "❚",
      tone: palette.warning,
    }
  }
  if (capture === "lost") {
    return { word: "capture lost", glyph: "○", tone: palette.danger }
  }
  if (capture === "black" || capture === "stale" || capture === "switching") {
    return { word: healthWord(capture), glyph: "◐", tone: palette.warning }
  }
  if (flashing) {
    return { word: "flashing scene", glyph: "◐", tone: palette.warning }
  }
  return { word: "syncing", glyph: "●", tone: palette.accent }
}

const SyncStrip = (props: SyncStripProps) => {
  const view = createMemo(() =>
    phaseView(props.status.phase, props.status.capture, props.status.flashing),
  )
  const color = createMemo(() => props.status.color)
  const colorLabel = createMemo(() => {
    const current = color()
    if (current === null) {
      return "—"
    }
    return current.mode === "temperature" ? `${current.kelvin ?? 4000} K` : current.hex
  })
  const swatchTone = createMemo(() => {
    const current = color()
    return current === null ? palette.faint : RGBA.fromHex(current.hex)
  })
  const brightness = createMemo(() =>
    props.status.syncBrightness === null ? "—" : `${props.status.syncBrightness}%`,
  )
  const frameAge = createMemo(() =>
    props.status.frameAgeMs === null ? "—" : `${props.status.frameAgeMs} ms`,
  )
  const restored = createMemo(
    () => Arr.filter(props.status.bulbs, (bulb) => bulb.status === "restored").length,
  )
  const notRestored = createMemo(() =>
    Arr.map(
      Arr.filter(props.status.bulbs, (bulb) => bulb.status === "not-restored"),
      (bulb) => bulb.name,
    ),
  )
  const restoreTail = createMemo(() => {
    if (props.status.phase !== "stopping") {
      return null
    }
    return restoreSummary(restored(), props.status.total, notRestored())
  })
  const restoreTone = createMemo(() => {
    if (notRestored().length > 0) {
      return palette.warning
    }
    return restored() === props.status.total ? palette.success : palette.muted
  })
  const keys = createMemo(() => syncKeys(props.status.phase))
  const monitor = createMemo(() =>
    truncate(props.status.monitor ?? "—", Math.max(8, Math.floor(props.width * 0.22))),
  )

  return (
    <box flexDirection="column" height={2} flexShrink={0}>
      <text height={1} truncate wrapMode="none">
        <span style={{ fg: view().tone }}>
          {view().glyph} {view().word}
        </span>
        <span style={{ fg: palette.faint }}> · </span>
        <span style={{ fg: palette.muted }}>{monitor()}</span>
        <span style={{ fg: props.status.selfCapture ? palette.warning : palette.muted }}>
          {props.status.selfCapture ? " ⚠ jyoti" : ""}
        </span>
        <span style={{ fg: palette.faint }}> · </span>
        <span style={{ fg: swatchTone() }}>██ {colorLabel()}</span>
        <span style={{ fg: restoreTail() === null ? palette.muted : restoreTone() }}>
          {" "}
          ·{" "}
          {restoreTail() ??
            `${brightness()} · frame ${frameAge()} · ${props.status.reachable}/${props.status.total}`}
        </span>
      </text>
      <text fg={palette.faint} height={1} truncate wrapMode="none">
        {keys()}
      </text>
    </box>
  )
}

export { SyncStrip }
