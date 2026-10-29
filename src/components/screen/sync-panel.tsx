import { RGBA } from "@opentui/core"
import { Show, createMemo } from "solid-js"

import type { SyncRow } from "@/components/screen/sync-rows"
import type { ScreenSyncModel } from "@/components/screen/use-sync"

import { truncate } from "@/lib/format"
import { isSyncRunning } from "@/lib/screen/sync/status"
import { palette } from "@/lib/theme"

interface SyncPanelProps {
  readonly model: ScreenSyncModel
  readonly selected: SyncRow
}

const Row = (props: {
  readonly label: string
  readonly value: string
  readonly selected: boolean
  readonly muted?: boolean
}) => (
  <box flexDirection="row" gap={1} height={1} flexShrink={0}>
    <text fg={props.selected ? palette.accent : palette.faint}>{props.selected ? "▸" : " "}</text>
    <box width={18} flexShrink={0}>
      <text fg={props.selected ? palette.text : palette.muted} truncate>
        {props.label}
      </text>
    </box>
    <text fg={props.muted === true ? palette.faint : palette.muted} truncate>
      {props.value}
    </text>
  </box>
)

const SyncPanel = (props: SyncPanelProps) => {
  const phase = createMemo(() => props.model.status().phase)
  const running = createMemo(() => isSyncRunning(phase()))
  const stopping = createMemo(() => phase() === "starting" || phase() === "stopping")
  const actionLabel = createMemo(() => {
    if (phase() === "starting") {
      return "Starting…"
    }
    if (phase() === "stopping") {
      return "Stopping…"
    }
    return running() ? "Stop screen sync" : "Start screen sync"
  })
  const pauseLabel = createMemo(() => (phase() === "paused" ? "Resume" : "Pause"))
  const monitor = createMemo(() =>
    truncate(
      props.model.status().pendingMonitor ?? props.model.status().monitor ?? "not chosen",
      30,
    ),
  )
  const gain = createMemo(() => `×${props.model.status().brightnessGain.toFixed(1)}`)
  const color = createMemo(() => props.model.status().color)
  const colorLabel = createMemo(() => color()?.hex ?? "—")
  const swatchTone = createMemo(() => {
    const current = color()
    return current === null ? palette.faint : RGBA.fromHex(current.hex)
  })
  const brightness = createMemo(() =>
    props.model.status().syncBrightness === null ? "—" : `${props.model.status().syncBrightness}%`,
  )

  return (
    <box flexDirection="column" gap={0} flexGrow={1} minHeight={0}>
      <Row
        label={actionLabel()}
        value={stopping() ? "wait" : "enter"}
        selected={props.selected === "action"}
        muted={stopping()}
      />
      <Show when={running()}>
        <Row label={pauseLabel()} value="enter" selected={props.selected === "pause"} />
      </Show>
      <Row label="Monitor" value={monitor()} selected={props.selected === "monitor"} />
      <Row label="Brightness gain" value={gain()} selected={props.selected === "gain"} />

      <box height={1} />

      <text fg={palette.faint} height={1} wrapMode="none" truncate>
        <span style={{ fg: swatchTone() }}>██ {colorLabel()}</span>
        <span>
          {" "}
          · {brightness()} · frame {props.model.status().frameAgeMs ?? "—"} ms ·{" "}
          {props.model.status().reachable}/{props.model.status().total} · {phase()}
        </span>
      </text>

      <text fg={palette.faint} height={1} wrapMode="none" truncate>
        ↑↓ row · ←→ adjust · enter act · esc close
      </text>
      <text fg={palette.faint} height={1} wrapMode="none" truncate>
        Gain changes brightness only; the colour is untouched.
      </text>
    </box>
  )
}

export { SyncPanel }
