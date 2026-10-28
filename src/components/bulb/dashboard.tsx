import { Array as Arr, String as Str } from "effect"
import { Show, createMemo } from "solid-js"

import type { ControlFocus } from "@/components/bulb/controls"
import type { BulbsModel } from "@/components/bulb/use-bulbs"
import type { Notice } from "@/components/ui/status"

import { AllDetail } from "@/components/bulb/all-detail"
import { Detail } from "@/components/bulb/detail"
import { Empty } from "@/components/bulb/empty"
import { Rail } from "@/components/bulb/rail"
import { StatusLine } from "@/components/ui/status"
import { aggregateState } from "@/lib/bulbs/aggregate"
import { palette } from "@/lib/theme"

type Pane = "bulbs" | "controls"

interface DashboardProps {
  readonly model: BulbsModel
  readonly focus: ControlFocus
  readonly pane: Pane
  readonly notice: Notice | null
  readonly width: number
  readonly height: number
  readonly onFocus: (focus: ControlFocus) => void
}

const Dashboard = (props: DashboardProps) => {
  const tooSmall = createMemo(() => props.width < 68 || props.height < 20)
  const shellWidth = createMemo(() => Math.min(108, Math.max(0, props.width - 2)))
  const railWidth = createMemo(() => (shellWidth() >= 96 ? 30 : 26))
  const detailWidth = createMemo(() => Math.min(76, Math.max(20, shellWidth() - railWidth() - 2)))
  const meterWidth = createMemo(() => Math.min(36, Math.max(10, detailWidth() - 24)))
  const mainHeight = createMemo(() => Math.max(0, props.height - 4))
  const aggregate = createMemo(() =>
    aggregateState(props.model.bulbs(), props.model.states(), props.model.stateErrors()),
  )
  const showingAll = createMemo(() => props.model.allSelected() && props.model.canSelectAll())

  const summary = createMemo(() => {
    if (props.model.loading()) {
      return "reading…"
    }
    const list = props.model.bulbs()
    if (list.length === 0) {
      return "no bulbs"
    }
    const on = Arr.filter(list, (bulb) => props.model.states().get(bulb.id)?.power === true).length
    return `${list.length} ${list.length === 1 ? "bulb" : "bulbs"} · ${on} on`
  })

  const hint = createMemo(() => {
    if (props.model.bulbs().length === 0) {
      return "no bulb selected"
    }
    if (props.model.selectedError() !== undefined) {
      return "r retry"
    }
    if (props.model.refreshing()) {
      return "refreshing…"
    }
    if (showingAll()) {
      return props.pane === "bulbs" ? "↑/↓ select · tab controls" : "space toggle all · tab bulbs"
    }
    if (props.pane === "bulbs") {
      return "↑/↓ select · tab controls"
    }
    switch (props.focus) {
      case "power": {
        return "space toggle · ↑/↓ control · tab bulbs"
      }
      case "brightness": {
        return "←/→ adjust · ↑/↓ control · tab bulbs"
      }
      case "temperature": {
        return "←/→ adjust · ↑/↓ control · tab bulbs"
      }
      case "color": {
        return "enter set color · ↑/↓ control · tab bulbs"
      }
      default: {
        return "tab bulbs"
      }
    }
  })

  const globalHint = createMemo(() => {
    if (props.model.bulbs().length === 0) {
      return "? help · q quit"
    }
    const all = props.model.canSelectAll() ? "A all · " : ""
    if (shellWidth() < 90) {
      return `${all}a add · ? help · q quit`
    }
    return `${all}a add · e edit · d remove · r refresh · ? help · q quit`
  })

  return (
    <Show
      when={!tooSmall()}
      fallback={
        <box
          flexGrow={1}
          flexDirection="column"
          alignItems="center"
          justifyContent="center"
          gap={1}
        >
          <text fg={palette.accent}>◉</text>
          <text fg={palette.text}>
            <strong>Give jyoti more room</strong>
          </text>
          <text fg={palette.muted}>The panel needs at least 68 columns and 20 rows.</text>
          <text fg={palette.faint}>
            {props.width} × {props.height} now
          </text>
        </box>
      }
    >
      <box flexGrow={1} flexDirection="row" justifyContent="center" minHeight={0}>
        <box width={shellWidth()} flexDirection="column" minWidth={0} minHeight={0}>
          <box flexDirection="column" flexShrink={0}>
            <box flexDirection="row" justifyContent="space-between" height={1}>
              <box flexDirection="row" gap={1}>
                <text fg={palette.accent}>
                  <strong>jyoti</strong>
                </text>
                <text fg={palette.faint}>xiaomi light control</text>
              </box>
              <text fg={palette.muted}>{summary()}</text>
            </box>
            <text fg={palette.border} height={1}>
              {Str.repeat(shellWidth())("─")}
            </text>
          </box>

          <box flexDirection="row" flexGrow={1} minHeight={0}>
            <Show
              when={!props.model.loading()}
              fallback={
                <box flexGrow={1} alignItems="center" justifyContent="center">
                  <text fg={palette.muted}>Reading bulbs…</text>
                </box>
              }
            >
              <Show when={props.model.bulbs().length > 0} fallback={<Empty />}>
                <box
                  width={railWidth()}
                  flexShrink={0}
                  border={["right"]}
                  borderColor={palette.border}
                  paddingRight={1}
                >
                  <Rail
                    bulbs={props.model.bulbs()}
                    states={props.model.states()}
                    stateErrors={props.model.stateErrors()}
                    aggregate={aggregate()}
                    selectedId={props.model.selectedId()}
                    allSelected={props.model.allSelected()}
                    canSelectAll={props.model.canSelectAll()}
                    height={mainHeight()}
                    width={railWidth() - 2}
                    onSelect={props.model.select}
                    onSelectAll={props.model.selectAll}
                    active={props.pane === "bulbs"}
                  />
                </box>

                <box flexGrow={1} paddingLeft={2} minWidth={0}>
                  <Show
                    when={showingAll()}
                    fallback={
                      <Show when={props.model.selected()}>
                        {(bulb) => (
                          <Detail
                            bulb={bulb()}
                            state={props.model.selectedState()}
                            error={props.model.selectedError()}
                            focus={props.focus}
                            active={props.pane === "controls"}
                            width={detailWidth()}
                            meterWidth={meterWidth()}
                            onFocus={props.onFocus}
                          />
                        )}
                      </Show>
                    }
                  >
                    <AllDetail
                      aggregate={aggregate()}
                      focus={props.focus}
                      active={props.pane === "controls"}
                      width={detailWidth()}
                      meterWidth={meterWidth()}
                      onFocus={props.onFocus}
                    />
                  </Show>
                </box>
              </Show>
            </Show>
          </box>

          <box flexDirection="column" flexShrink={0}>
            <text fg={palette.border} height={1}>
              {Str.repeat(shellWidth())("─")}
            </text>
            <box flexDirection="row" justifyContent="space-between" height={1} gap={2}>
              <StatusLine notice={props.notice} fallback={hint()} />
              <text fg={palette.muted} truncate wrapMode="none" flexShrink={0}>
                {globalHint()}
              </text>
            </box>
          </box>
        </box>
      </box>
    </Show>
  )
}

export { Dashboard, type Pane }
