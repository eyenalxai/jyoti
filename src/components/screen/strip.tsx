import { RGBA } from "@opentui/core"
import { createMemo } from "solid-js"

import type { CaptureHealth, SyncPhase, SyncStatus } from "@/lib/screen/sync/status"

import { palette } from "@/lib/theme"

interface SyncStripProps {
  readonly status: SyncStatus
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

const phaseWord = (phase: SyncPhase, capture: CaptureHealth): string => {
  if (phase === "starting") {
    return "starting"
  }
  if (phase === "stopping") {
    return "stopping"
  }
  if (phase === "paused") {
    const health = healthWord(capture)
    return health.length === 0 ? "paused" : `paused · ${health}`
  }
  const health = healthWord(capture)
  return health.length === 0 ? "syncing" : health
}

const phaseGlyph = (phase: SyncPhase, capture: CaptureHealth): string => {
  if (phase === "starting" || phase === "stopping") {
    return "◌"
  }
  if (phase === "paused") {
    return "❚"
  }
  if (capture === "lost") {
    return "○"
  }
  if (capture === "black" || capture === "stale" || capture === "switching") {
    return "◐"
  }
  return "●"
}

const phaseTone = (phase: SyncPhase, capture: CaptureHealth): RGBA => {
  if (phase === "paused") {
    return palette.warning
  }
  if (capture === "lost") {
    return palette.danger
  }
  if (capture === "black" || capture === "stale" || capture === "switching") {
    return palette.warning
  }
  if (phase === "syncing") {
    return palette.accent
  }
  return palette.muted
}

const runKeys = (phase: SyncPhase): string => {
  if (phase === "starting") {
    return "s cancel · m monitor"
  }
  if (phase === "paused") {
    return "s stop · x resume · m monitor"
  }
  if (phase === "stopping") {
    return ""
  }
  return "s stop · x pause · m monitor"
}

const SyncStrip = (props: SyncStripProps) => {
  const word = createMemo(() => phaseWord(props.status.phase, props.status.capture))
  const glyph = createMemo(() => phaseGlyph(props.status.phase, props.status.capture))
  const tone = createMemo(() => phaseTone(props.status.phase, props.status.capture))
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
  const keys = createMemo(() => runKeys(props.status.phase))

  return (
    <box flexDirection="column" height={2} flexShrink={0}>
      <text height={1} truncate wrapMode="none">
        <span style={{ fg: tone() }}>
          {glyph()} {word()}
        </span>
        <span style={{ fg: palette.faint }}> · </span>
        <span style={{ fg: palette.muted }}>{props.status.monitor ?? "—"}</span>
        <span style={{ fg: props.status.selfCapture ? palette.warning : palette.muted }}>
          {props.status.selfCapture ? " ⚠ jyoti" : ""}
        </span>
        <span style={{ fg: palette.faint }}> · </span>
        <span style={{ fg: swatchTone() }}>██ {colorLabel()}</span>
        <span style={{ fg: palette.muted }}>
          {" "}
          · {brightness()} · frame {frameAge()} · {props.status.reachable}/{props.status.total}
        </span>
        <span style={{ fg: palette.danger }}>
          {props.status.error === null ? "" : " · rejected"}
        </span>
      </text>
      <text fg={palette.faint} height={1} truncate wrapMode="none">
        {keys()}
      </text>
    </box>
  )
}

export { SyncStrip }
