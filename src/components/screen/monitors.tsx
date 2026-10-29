import { For, Show, createMemo } from "solid-js"

import type { Monitor } from "@/lib/screen/frame"

import { palette } from "@/lib/theme"

interface MonitorsProps {
  readonly monitors: readonly Monitor[]
  readonly selected: number
  readonly captured: string | null
  readonly pending: string | null
  readonly busy: boolean
}

const VISIBLE_MONITORS = 6

const focusWord = (monitor: Monitor): string => {
  if (monitor.focused === true) {
    return "focused"
  }
  if (monitor.focused === null) {
    return "focus ?"
  }
  return ""
}

const Monitors = (props: MonitorsProps) => {
  const captured = createMemo(() => props.captured)
  const pending = createMemo(() => props.pending)
  const start = createMemo(() =>
    Math.max(
      0,
      Math.min(
        props.monitors.length - VISIBLE_MONITORS,
        props.selected - Math.floor(VISIBLE_MONITORS / 2),
      ),
    ),
  )
  const visible = createMemo(() => props.monitors.slice(start(), start() + VISIBLE_MONITORS))

  return (
    <box flexDirection="column" gap={0} flexGrow={1} minHeight={0}>
      <Show
        when={props.monitors.length > 0}
        fallback={
          <text fg={palette.muted}>{props.busy ? "Reading monitors…" : "No monitor listed."}</text>
        }
      >
        <For each={visible()}>
          {(monitor, index) => {
            const selected = () => start() + index() === props.selected
            return (
              <box flexDirection="row" gap={1} height={1} flexShrink={0}>
                <text fg={selected() ? palette.accent : palette.faint}>
                  {selected() ? "▸" : " "}
                </text>
                <box width={18} flexShrink={0}>
                  <text fg={selected() ? palette.text : palette.muted} truncate>
                    {monitor.name}
                  </text>
                </box>
                <text fg={palette.muted}>
                  {monitor.width}×{monitor.height}
                </text>
                <text fg={palette.faint}>{Math.round(monitor.refreshHz)} Hz</text>
                <Show when={focusWord(monitor).length > 0}>
                  <text fg={palette.warning}>{focusWord(monitor)}</text>
                </Show>
                <Show when={captured() === monitor.name}>
                  <text fg={palette.success}>captured</text>
                </Show>
                <Show when={pending() === monitor.name}>
                  <text fg={palette.warning}>next run</text>
                </Show>
              </box>
            )
          }}
        </For>
      </Show>

      <box height={1} />

      <text fg={palette.faint} height={1} flexShrink={0}>
        The focus marker is the monitor jyoti is on. Its warning stays in the strip.
      </text>
      <text fg={palette.faint} height={1} flexShrink={0}>
        While stopped a choice applies to the next run. During a run it switches now.
      </text>
      <text fg={palette.faint} height={1} flexShrink={0}>
        ↑/↓ monitor · enter choose · esc cancel
      </text>
    </box>
  )
}

export { Monitors }
