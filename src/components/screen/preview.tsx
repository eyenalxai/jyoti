import { RGBA } from "@opentui/core"
import { For, Show } from "solid-js"

import type { CapturePreviewModel } from "@/components/screen/use-preview"

import { palette } from "@/lib/theme"

interface PreviewProps {
  readonly model: CapturePreviewModel
}

const Preview = (props: PreviewProps) => {
  const derived = () => props.model.derived()
  const frame = () => props.model.frame()

  return (
    <box flexDirection="column" gap={0} flexGrow={1} minHeight={0}>
      <text fg={palette.faint} height={1} flexShrink={0}>
        monitors
      </text>
      <Show
        when={props.model.monitors().length > 0}
        fallback={<text fg={palette.muted}>No monitor listed.</text>}
      >
        <For each={props.model.monitors()}>
          {(monitor, index) => (
            <box flexDirection="row" gap={1} height={1} flexShrink={0}>
              <text fg={index() === props.model.selected() ? palette.accent : palette.faint}>
                {index() === props.model.selected() ? "▸" : " "}
              </text>
              <box width={12} flexShrink={0}>
                <text fg={index() === props.model.selected() ? palette.text : palette.muted}>
                  {monitor.name}
                </text>
              </box>
              <text fg={palette.muted}>
                {monitor.width}×{monitor.height}
              </text>
              <text fg={palette.faint}>{Math.round(monitor.refreshHz)} Hz</text>
              <Show when={monitor.focused === true}>
                <text fg={palette.warning}>focused</text>
              </Show>
            </box>
          )}
        </For>
      </Show>

      <text fg={palette.faint} height={1} flexShrink={0}>
        capture
      </text>
      <box flexDirection="row" gap={1} height={1} flexShrink={0}>
        <text fg={props.model.running() ? palette.success : palette.muted}>
          {props.model.busy() ? "starting…" : props.model.running() ? "capturing" : "stopped"}
        </text>
        <Show when={props.model.monitors()[props.model.selected()]}>
          {(monitor) => <text fg={palette.muted}>{monitor().name}</text>}
        </Show>
      </box>

      <Show when={frame()?.blackout === true}>
        <text fg={palette.warning}>black frame · hold</text>
      </Show>

      <Show
        when={derived()}
        fallback={
          <text fg={palette.faint}>
            {props.model.loading()
              ? "reading monitors…"
              : props.model.running()
                ? "waiting for a colour frame…"
                : "Start the capture to derive a colour."}
          </text>
        }
      >
        {(color) => (
          <>
            <box flexDirection="row" gap={1} height={1} flexShrink={0}>
              <text fg={RGBA.fromHex(color().hex)}>██████</text>
              <text fg={palette.text}>{color().hex}</text>
              <Show when={color().kelvin !== null}>
                <text fg={palette.muted}>{color().kelvin} K</text>
              </Show>
              <text fg={palette.faint}>{color().mode}</text>
            </box>
            <box flexDirection="row" gap={2} height={1} flexShrink={0}>
              <text fg={palette.muted}>scene {Math.round(color().sceneL * 100)}%</text>
              <Show when={frame()}>
                {(current) => (
                  <>
                    <text fg={palette.muted}>frame {current().ageMs} ms</text>
                    <text fg={palette.faint}>#{current().sequence}</text>
                    <text fg={palette.faint}>p90 {current().p90Chroma.toFixed(3)}</text>
                    <Show when={!current().published && !current().blackout}>
                      <text fg={palette.warning}>deadband</text>
                    </Show>
                  </>
                )}
              </Show>
            </box>
          </>
        )}
      </Show>

      <Show when={props.model.error()}>
        {(message) => <text fg={palette.danger}>{message()}</text>}
      </Show>

      <text fg={palette.faint} height={1} flexShrink={0}>
        ↑/↓ monitor · enter start or stop · esc close
      </text>
    </box>
  )
}

export { Preview }
