import { clamp } from "@/lib/format"

interface DashboardLayout {
  readonly tooSmall: boolean
  readonly split: boolean
  readonly railWidth: number
  readonly detailWidth: number
  readonly labelWidth: number
  readonly valueWidth: number
  readonly meterWidth: number
  readonly controlWidth: number
  readonly rowHints: boolean
}

const MIN_WIDTH = 40
const MIN_HEIGHT = 14
const SPLIT_MIN = 72
const METER_MIN = 8
const METER_MAX = 48
const CONTROL_MAX = 96
const VALUE_WIDTH = 10
const CARET_WIDTH = 1
const HINT_WIDTH = 11
const GAP = 1
const ROOT_PAD = 2
const RAIL_GAP = 2

const railFor = (width: number): number => {
  if (width >= 140) {
    return 34
  }
  if (width >= 100) {
    return 30
  }
  return 26
}

const layoutFor = (width: number, height: number): DashboardLayout => {
  const split = width >= SPLIT_MIN
  const railWidth = split ? railFor(width) : 0
  const inner = Math.max(0, width - ROOT_PAD)
  const detailBox = split ? Math.max(0, inner - railWidth - RAIL_GAP) : inner
  const detailWidth = Math.max(0, detailBox)
  const labelWidth = detailWidth >= 52 ? 12 : 10
  const rowHints = detailWidth >= 62
  const fixed =
    CARET_WIDTH + GAP + labelWidth + GAP + VALUE_WIDTH + GAP + (rowHints ? HINT_WIDTH + GAP : 0)
  const meterWidth = clamp(detailWidth - fixed, METER_MIN, METER_MAX)

  return {
    tooSmall: width < MIN_WIDTH || height < MIN_HEIGHT,
    split,
    railWidth,
    detailWidth,
    labelWidth,
    valueWidth: VALUE_WIDTH,
    meterWidth,
    controlWidth: Math.min(detailWidth, CONTROL_MAX),
    rowHints,
  }
}

export { layoutFor, type DashboardLayout }
