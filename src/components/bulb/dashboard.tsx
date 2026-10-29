import type { BoxRenderable } from "@opentui/core"

import { Array as Arr } from "effect"
import { Show, createMemo, createSignal } from "solid-js"

import type { ControlFocus } from "@/components/bulb/controls"
import type { Pane } from "@/components/bulb/pane"
import type { BulbsModel } from "@/components/bulb/use-bulbs"
import type { LightSide } from "@/lib/bulbs/aggregate"
import type { BulbId } from "@/lib/bulbs/schema"
import type { BulbSyncPhase, SyncStatus } from "@/lib/screen/sync/status"

import { AllDetail } from "@/components/bulb/all-detail"
import { Detail } from "@/components/bulb/detail"
import { Empty } from "@/components/bulb/empty"
import { footerHints } from "@/components/bulb/hints"
import { layoutFor } from "@/components/bulb/layout"
import { Rail } from "@/components/bulb/rail"
import { SyncStrip } from "@/components/screen/strip"
import { KeyBar } from "@/components/ui/key"
import { Rule } from "@/components/ui/rule"
import { aggregateState } from "@/lib/bulbs/aggregate"
import { palette } from "@/lib/theme"

interface DashboardProps {
  readonly model: BulbsModel
  readonly focus: ControlFocus
  readonly pane: Pane
  readonly syncStatus: SyncStatus
  readonly width: number
  readonly height: number
  readonly colorOpen: boolean
  readonly colorBusy: boolean
  readonly colorInitial: string
  readonly onFocus: (focus: ControlFocus) => void
  readonly onQuality: (quality: LightSide) => void
  readonly onPreviewColor: (hex: string) => Promise<boolean>
  readonly onSubmitColor: (hex: string) => void
  readonly onDismissColor: () => void
}

const Dashboard = (props: DashboardProps) => {
  const layout = createMemo(() => layoutFor(props.width, props.height))
  const [measuredRail, setMeasuredRail] = createSignal(0)
  const [railHost, setRailHost] = createSignal<BoxRenderable>()

  const syncActive = createMemo(() => props.syncStatus.phase !== "idle")
  const aggregate = createMemo(() =>
    aggregateState(props.model.bulbs(), props.model.states(), props.model.stateErrors()),
  )
  const showingAll = createMemo(() => props.model.allSelected() && props.model.canSelectAll())

  const railHeight = createMemo(() => {
    const measured = measuredRail()
    return measured > 0 ? measured : Math.max(2, props.height - 8)
  })

  const railColumn = createMemo(() => (layout().split ? layout().railWidth : props.width - 2))
  const railContent = createMemo(() =>
    layout().split ? Math.max(8, railColumn() - 2) : Math.max(8, railColumn()),
  )

  const syncBulbs = createMemo(() => {
    const map = new Map<BulbId, BulbSyncPhase>()
    for (const bulb of props.syncStatus.bulbs) {
      map.set(bulb.id, bulb.status)
    }
    return map
  })

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

  const hints = createMemo(() =>
    footerHints({
      pane: props.pane,
      hasBulbs: props.model.bulbs().length > 0,
      canSelectAll: props.model.canSelectAll(),
      error: props.model.selectedError() !== undefined,
      syncActive: syncActive(),
    }),
  )

  const showRail = createMemo(() => layout().split || props.pane === "bulbs")
  const showDetail = createMemo(() => layout().split || props.pane === "controls")

  return (
    <Show
      when={!layout().tooSmall}
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
          <text fg={palette.muted}>The panel needs at least 40 columns and 14 rows.</text>
          <text fg={palette.faint}>
            {props.width} × {props.height} now
          </text>
        </box>
      }
    >
      <box flexDirection="column" flexGrow={1} minHeight={0} width="100%">
        <box flexDirection="row" justifyContent="space-between" height={1} gap={2}>
          <box flexDirection="row" gap={1} minWidth={0}>
            <text fg={palette.accent} wrapMode="none">
              <strong>jyoti</strong>
            </text>
            <Show when={props.width >= 60}>
              <text fg={palette.faint} wrapMode="none">
                xiaomi light control
              </text>
            </Show>
          </box>
          <text fg={palette.muted} flexShrink={0} wrapMode="none">
            {summary()}
          </text>
        </box>
        <Rule width={props.width - 2} />

        <box flexDirection="row" flexGrow={1} minHeight={0} width="100%">
          <Show
            when={!props.model.loading()}
            fallback={
              <box flexGrow={1} alignItems="center" justifyContent="center">
                <text fg={palette.muted}>Reading bulbs…</text>
              </box>
            }
          >
            <Show when={props.model.bulbs().length > 0} fallback={<Empty />}>
              <Show when={showRail()}>
                <box
                  ref={(element) => {
                    setRailHost(element)
                  }}
                  onSizeChange={() => {
                    setMeasuredRail(railHost()?.height ?? 0)
                  }}
                  width={railColumn()}
                  flexGrow={layout().split ? 0 : 1}
                  flexShrink={0}
                  flexDirection="column"
                  minWidth={0}
                  minHeight={0}
                  border={layout().split ? ["right"] : []}
                  borderColor={palette.border}
                  paddingRight={layout().split ? 1 : 0}
                >
                  <Rail
                    bulbs={props.model.bulbs()}
                    states={props.model.states()}
                    stateErrors={props.model.stateErrors()}
                    aggregate={aggregate()}
                    selectedId={props.model.selectedId()}
                    allSelected={props.model.allSelected()}
                    canSelectAll={props.model.canSelectAll()}
                    syncBulbs={syncBulbs()}
                    height={railHeight()}
                    width={railContent()}
                    onSelect={props.model.select}
                    onSelectAll={props.model.selectAll}
                    active={props.pane === "bulbs"}
                  />
                </box>
              </Show>

              <Show when={showDetail()}>
                <box
                  flexGrow={1}
                  flexDirection="column"
                  minWidth={0}
                  minHeight={0}
                  paddingLeft={layout().split ? 2 : 0}
                >
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
                            syncLocked={syncActive()}
                            width={layout().detailWidth}
                            meterWidth={layout().meterWidth}
                            labelWidth={layout().labelWidth}
                            valueWidth={layout().valueWidth}
                            controlWidth={layout().controlWidth}
                            rowHints={layout().rowHints}
                            colorOpen={props.colorOpen}
                            colorBusy={props.colorBusy}
                            colorInitial={props.colorInitial}
                            onFocus={props.onFocus}
                            onQuality={props.onQuality}
                            onPreviewColor={props.onPreviewColor}
                            onSubmitColor={props.onSubmitColor}
                            onDismissColor={props.onDismissColor}
                          />
                        )}
                      </Show>
                    }
                  >
                    <AllDetail
                      aggregate={aggregate()}
                      focus={props.focus}
                      active={props.pane === "controls"}
                      syncLocked={syncActive()}
                      width={layout().detailWidth}
                      meterWidth={layout().meterWidth}
                      labelWidth={layout().labelWidth}
                      valueWidth={layout().valueWidth}
                      controlWidth={layout().controlWidth}
                      rowHints={layout().rowHints}
                      colorOpen={props.colorOpen}
                      colorBusy={props.colorBusy}
                      colorInitial={props.colorInitial}
                      onFocus={props.onFocus}
                      onQuality={props.onQuality}
                      onPreviewColor={props.onPreviewColor}
                      onSubmitColor={props.onSubmitColor}
                      onDismissColor={props.onDismissColor}
                    />
                  </Show>
                </box>
              </Show>
            </Show>
          </Show>
        </box>

        <Show when={syncActive()}>
          <SyncStrip status={props.syncStatus} width={props.width - 2} />
        </Show>

        <Show when={!props.colorOpen}>
          <box flexDirection="column" flexShrink={0} width="100%">
            <Rule width={props.width - 2} />
            <KeyBar hints={hints()} />
          </box>
        </Show>
      </box>
    </Show>
  )
}

export { Dashboard }
