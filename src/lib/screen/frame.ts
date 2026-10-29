interface Monitor {
  readonly name: string
  readonly description: string
  readonly width: number
  readonly height: number
  readonly refreshHz: number
  readonly focused: boolean | null
}

interface ScreenFrame {
  readonly pixels: Uint8Array
  readonly width: number
  readonly height: number
  readonly sequence: number
  readonly ageMs: number
}

interface GridSize {
  readonly width: number
  readonly height: number
}

const DEFAULT_GRID: GridSize = { width: 128, height: 72 }
const DEFAULT_RATE_HZ = 10

export { DEFAULT_GRID, DEFAULT_RATE_HZ }
export type { GridSize, Monitor, ScreenFrame }
