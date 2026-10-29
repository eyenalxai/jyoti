import { For, Show, createMemo } from "solid-js"

import type { Monitor } from "@/lib/screen/frame"

import { palette } from "@/lib/theme"

interface MonitorsProps {
  readonly monitors: readonly Monitor[]
  readonly selected: number
  readonly current: string | null
  readonly busy: boolean
  readonly error: string | null
}

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
  const current = createMemo(() => props.current)

  return (
    <box flexDirection="column" gap={0} flexGrow={1} minHeight={0}>
      <Show
        when={props.monitors.length > 0}
        fallback={
          <text fg={palette.muted}>{props.busy ? "Reading monitors…" : "No monitor listed."}</text>
        }
      >
        <For each={props.monitors}>
          {(monitor, index) => (
            <box flexDirection="row" gap={1} height={1} flexShrink={0}>
              <text fg={index() === props.selected ? palette.accent : palette.faint}>
                {index() === props.selected ? "▸" : " "}
              </text>
              <box width={18} flexShrink={0}>
                <text fg={index() === props.selected ? palette.text : palette.muted} truncate>
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
              <Show when={current() === monitor.name}>
                <text fg={palette.success}>captured</text>
              </Show>
            </box>
          )}
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

      <Show when={props.error !== null}>
        {(message) => (
          <text fg={palette.danger} truncate wrapMode="none">
            ✗ {message()}
          </text>
        )}
      </Show>
    </box>
  )
}

export { Monitors }
