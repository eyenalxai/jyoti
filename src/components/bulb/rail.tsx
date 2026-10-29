import { Array as Arr, Option } from "effect"
import { For, Show, createMemo } from "solid-js"

import type { AggregateState } from "@/lib/bulbs/aggregate"
import type { Bulb, BulbId, BulbState } from "@/lib/bulbs/schema"
import type { BulbSyncPhase } from "@/lib/screen/sync/status"

import { aggregateGlyph, aggregateShort, powerGlyph, shortState, truncate } from "@/lib/format"
import { palette } from "@/lib/theme"

interface RailProps {
  readonly bulbs: readonly Bulb[]
  readonly states: ReadonlyMap<string, BulbState>
  readonly stateErrors: ReadonlyMap<string, string>
  readonly aggregate: AggregateState
  readonly selectedId: BulbId | null
  readonly allSelected: boolean
  readonly canSelectAll: boolean
  readonly syncBulbs: ReadonlyMap<BulbId, BulbSyncPhase>
  readonly height: number
  readonly width: number
  readonly active: boolean
  readonly onSelect: (id: BulbId) => void
  readonly onSelectAll: () => void
}

const syncGlyph = (status: BulbSyncPhase): string => {
  if (status === "syncing") {
    return "●"
  }
  if (status === "unreachable") {
    return "○"
  }
  if (status === "restored") {
    return "✓"
  }
  return "✗"
}

const syncTone = (status: BulbSyncPhase) => {
  if (status === "syncing" || status === "restored") {
    return palette.success
  }
  return palette.danger
}

const Rail = (props: RailProps) => {
  const allRows = createMemo(() => (props.canSelectAll ? 2 : 0))
  const rows = createMemo(() => Math.max(1, Math.floor((props.height - 2 - allRows()) / 2)))
  const selectedIndex = createMemo(() => {
    if (props.allSelected) {
      return 0
    }
    return Math.max(
      0,
      Option.getOrElse(
        Arr.findFirstIndex(props.bulbs, (bulb) => bulb.id === props.selectedId),
        () => -1,
      ),
    )
  })
  const start = createMemo(() => {
    const maxStart = Math.max(0, props.bulbs.length - rows())
    return Math.min(maxStart, Math.max(0, selectedIndex() - Math.floor(rows() / 2)))
  })
  const visible = createMemo(() => props.bulbs.slice(start(), start() + rows()))

  const allGlyphTone = () => {
    if (props.aggregate.reachable === 0 && props.aggregate.failed > 0) {
      return palette.danger
    }
    if (props.aggregate.power === "on") {
      return palette.accent
    }
    if (props.aggregate.power === "mixed") {
      return palette.warning
    }
    return palette.faint
  }

  const allDetailTone = () => {
    if (props.aggregate.failed === 0) {
      return palette.muted
    }
    return props.aggregate.reachable === 0 ? palette.danger : palette.warning
  }

  return (
    <box flexDirection="column" height={props.height} flexShrink={0}>
      <box flexDirection="row" justifyContent="space-between" height={1}>
        <text fg={props.active ? palette.accent : palette.muted}>bulbs</text>
        <text fg={palette.faint}>{props.bulbs.length}</text>
      </box>

      <Show when={props.canSelectAll}>
        <box
          flexDirection="column"
          height={2}
          onMouseDown={() => {
            props.onSelectAll()
          }}
        >
          <box flexDirection="row" gap={1} height={1}>
            <text
              fg={
                props.allSelected
                  ? props.active
                    ? palette.accent
                    : palette.muted
                  : palette.background
              }
            >
              {props.allSelected ? "▌" : " "}
            </text>
            <text
              fg={props.allSelected ? (props.active ? palette.accent : palette.text) : palette.text}
              truncate
              wrapMode="none"
            >
              <span style={{ fg: allGlyphTone() }}>{aggregateGlyph(props.aggregate.power)}</span>{" "}
              All bulbs
            </text>
          </box>
          <box flexDirection="row" paddingLeft={2} height={1}>
            <text fg={allDetailTone()} truncate wrapMode="none">
              {truncate(aggregateShort(props.aggregate), Math.max(4, props.width - 5))}
            </text>
          </box>
        </box>
      </Show>

      <box flexDirection="column" flexGrow={1} minHeight={0}>
        <For each={visible()}>
          {(bulb) => {
            const selected = () => bulb.id === props.selectedId
            const state = () => props.states.get(bulb.id)
            const error = () => props.stateErrors.get(bulb.id)
            const sync = () => props.syncBulbs.get(bulb.id)
            const glyphColor = () => {
              if (error() !== undefined) {
                return palette.danger
              }
              if (state()?.power === true) {
                return palette.accent
              }
              return palette.faint
            }
            const detail = () => {
              if (error() !== undefined) {
                return "unreachable · r retry"
              }
              return shortState(state())
            }

            return (
              <box
                flexDirection="column"
                height={2}
                onMouseDown={() => {
                  props.onSelect(bulb.id)
                }}
              >
                <box flexDirection="row" gap={1} height={1}>
                  <text
                    fg={
                      selected()
                        ? props.active
                          ? palette.accent
                          : palette.muted
                        : palette.background
                    }
                  >
                    {selected() ? "▌" : " "}
                  </text>
                  <text
                    fg={selected() ? (props.active ? palette.accent : palette.text) : palette.text}
                    truncate
                    wrapMode="none"
                  >
                    <span style={{ fg: glyphColor() }}>{powerGlyph(state())}</span>{" "}
                    {truncate(bulb.name, Math.max(4, props.width - 6))}
                    <Show when={sync()}>
                      {(status) => (
                        <span style={{ fg: syncTone(status()) }}> {syncGlyph(status())}</span>
                      )}
                    </Show>
                  </text>
                </box>
                <box flexDirection="row" paddingLeft={2} height={1}>
                  <text fg={palette.muted} truncate wrapMode="none">
                    {truncate(detail(), Math.max(4, props.width - 5))}
                  </text>
                </box>
              </box>
            )
          }}
        </For>
      </box>
    </box>
  )
}

export { Rail }
