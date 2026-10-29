import { Array as Arr, String as Str } from "effect"
import { Show, createMemo } from "solid-js"

import type { ControlFocus } from "@/components/bulb/controls"
import type { Pane } from "@/components/bulb/pane"
import type { BulbsModel } from "@/components/bulb/use-bulbs"
import type { LightSide } from "@/lib/bulbs/aggregate"
import type { BulbId } from "@/lib/bulbs/schema"
import type { BulbSyncPhase, SyncStatus } from "@/lib/screen/sync/status"

import { AllDetail } from "@/components/bulb/all-detail"
import { Detail } from "@/components/bulb/detail"
import { Empty } from "@/components/bulb/empty"
import { dashboardHint, globalHint as globalHintText } from "@/components/bulb/hints"
import { Rail } from "@/components/bulb/rail"
import { SyncStrip } from "@/components/screen/strip"
import { aggregateState, lightQualityOf } from "@/lib/bulbs/aggregate"
import { syncKeys } from "@/lib/screen/sync/copy"
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
  const tooSmall = createMemo(() => props.width < 68 || props.height < 20)
  const shellWidth = createMemo(() => Math.min(108, Math.max(0, props.width - 2)))
  const railWidth = createMemo(() => (shellWidth() >= 96 ? 30 : 26))
  const detailWidth = createMemo(() => Math.min(76, Math.max(20, shellWidth() - railWidth() - 2)))
  const meterWidth = createMemo(() => Math.min(36, Math.max(10, detailWidth() - 24)))
  const syncActive = createMemo(() => props.syncStatus.phase !== "idle")
  const mainHeight = createMemo(() => Math.max(0, props.height - (syncActive() ? 6 : 4)))
  const aggregate = createMemo(() =>
    aggregateState(props.model.bulbs(), props.model.states(), props.model.stateErrors()),
  )
  const showingAll = createMemo(() => props.model.allSelected() && props.model.canSelectAll())
  const quality = createMemo(() =>
    props.model.allSelected()
      ? aggregate().quality
      : lightQualityOf(props.model.selectedState()?.mode),
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

  const hint = createMemo(() =>
    dashboardHint({
      pane: props.pane,
      focus: props.focus,
      quality: quality(),
      showingAll: showingAll(),
      hasBulbs: props.model.bulbs().length > 0,
      error: props.model.selectedError() !== undefined,
      refreshing: props.model.refreshing(),
      syncActive: syncActive(),
    }),
  )

  const syncHint = createMemo(() => syncKeys(props.syncStatus.phase))

  const footerHint = createMemo(() =>
    globalHintText({
      syncActive: syncActive(),
      shellWidth: shellWidth(),
      hasBulbs: props.model.bulbs().length > 0,
      canSelectAll: props.model.canSelectAll(),
      syncHint: syncHint(),
    }),
  )

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
                    syncBulbs={syncBulbs()}
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
                            syncLocked={syncActive()}
                            width={detailWidth()}
                            meterWidth={meterWidth()}
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
                      width={detailWidth()}
                      meterWidth={meterWidth()}
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
          </box>

          <Show when={syncActive()}>
            <SyncStrip status={props.syncStatus} width={shellWidth()} />
          </Show>

          <box flexDirection="column" flexShrink={0}>
            <text fg={palette.border} height={1}>
              {Str.repeat(shellWidth())("─")}
            </text>
            <box flexDirection="row" justifyContent="space-between" height={1} gap={2}>
              <text fg={palette.muted} truncate wrapMode="none" flexGrow={1}>
                {hint()}
              </text>
              <text fg={palette.muted} truncate wrapMode="none" flexShrink={0}>
                {footerHint()}
              </text>
            </box>
          </box>
        </box>
      </box>
    </Show>
  )
}

export { Dashboard }
