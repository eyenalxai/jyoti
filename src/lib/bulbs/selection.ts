import { Array as Arr, Option } from "effect"

import type { Bulb, BulbId } from "@/lib/bulbs/schema"

type BulbSelection = { readonly kind: "all" } | { readonly kind: "bulb"; readonly id: BulbId }

const allSelection: BulbSelection = { kind: "all" }

const bulbSelection = (id: BulbId): BulbSelection => ({ kind: "bulb", id })

const resolveSelection = (
  bulbs: readonly Bulb[],
  previous: BulbSelection | null,
): BulbSelection | null => {
  if (previous?.kind === "all" && bulbs.length >= 2) {
    return allSelection
  }
  const kept =
    previous?.kind === "bulb"
      ? Option.getOrNull(Arr.findFirst(bulbs, (bulb) => bulb.id === previous.id))
      : undefined
  const fallback = kept ?? bulbs[0]
  return fallback === undefined ? null : bulbSelection(fallback.id)
}

const moveSelection = (
  bulbs: readonly Bulb[],
  current: BulbSelection | null,
  delta: number,
): BulbSelection | null => {
  if (bulbs.length === 0) {
    return null
  }
  const offset = bulbs.length >= 2 ? 1 : 0
  const bulbIndex =
    current?.kind === "bulb"
      ? Option.getOrElse(
          Arr.findFirstIndex(bulbs, (bulb) => bulb.id === current.id),
          () => 0,
        )
      : 0
  const position = current === null ? -1 : current.kind === "all" ? 0 : bulbIndex + offset
  const count = bulbs.length + offset
  const index = Math.min(count - 1, Math.max(0, position + delta))
  if (offset === 1 && index === 0) {
    return allSelection
  }
  const bulb = bulbs[index - offset]
  return bulb === undefined ? current : bulbSelection(bulb.id)
}

export { allSelection, bulbSelection, moveSelection, resolveSelection, type BulbSelection }
