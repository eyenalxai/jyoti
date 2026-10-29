import type { ScreenFrame } from "@/lib/screen/frame"

import { rgbToOklch } from "@/lib/color"

const BAR_DARK_THRESHOLD = 8
const BAR_COVERAGE = 0.97
const MINIMUM_SURVIVAL = 0.25

interface ContentRect {
  readonly x: number
  readonly y: number
  readonly width: number
  readonly height: number
}

interface PreparedFrame {
  readonly width: number
  readonly height: number
  readonly rect: ContentRect
  readonly pixelCount: number
  readonly lab: Float32Array
  readonly chroma: Float32Array
  readonly meanL: number
  readonly p90Chroma: number
  readonly blackout: boolean
}

const isNearBlack = (pixels: Uint8Array, offset: number): boolean =>
  (pixels[offset] ?? 0) <= BAR_DARK_THRESHOLD &&
  (pixels[offset + 1] ?? 0) <= BAR_DARK_THRESHOLD &&
  (pixels[offset + 2] ?? 0) <= BAR_DARK_THRESHOLD

const darkFraction = (
  pixels: Uint8Array,
  width: number,
  height: number,
  axis: "row" | "column",
  index: number,
): number => {
  const count = axis === "row" ? width : height
  let dark = 0
  for (let cursor = 0; cursor < count; cursor += 1) {
    const x = axis === "row" ? cursor : index
    const y = axis === "row" ? index : cursor
    if (isNearBlack(pixels, (y * width + x) * 4)) {
      dark += 1
    }
  }
  return dark / count
}

const detectContentRect = (
  frame: ScreenFrame,
): { readonly rect: ContentRect; readonly rejected: boolean } => {
  const { width, height, pixels } = frame
  let top = 0
  let bottom = height - 1
  let left = 0
  let right = width - 1

  while (top < height && darkFraction(pixels, width, height, "row", top) >= BAR_COVERAGE) {
    top += 1
  }
  while (bottom >= top && darkFraction(pixels, width, height, "row", bottom) >= BAR_COVERAGE) {
    bottom -= 1
  }
  while (left < width && darkFraction(pixels, width, height, "column", left) >= BAR_COVERAGE) {
    left += 1
  }
  while (right >= left && darkFraction(pixels, width, height, "column", right) >= BAR_COVERAGE) {
    right -= 1
  }

  const candidateWidth = right - left + 1
  const candidateHeight = bottom - top + 1
  if (
    candidateWidth <= 0 ||
    candidateHeight <= 0 ||
    candidateWidth * candidateHeight < width * height * MINIMUM_SURVIVAL
  ) {
    return { rect: { x: 0, y: 0, width, height }, rejected: true }
  }
  return {
    rect: { x: left, y: top, width: candidateWidth, height: candidateHeight },
    rejected: false,
  }
}

const fullDarkFraction = (frame: ScreenFrame): number => {
  const { pixels } = frame
  const total = pixels.length / 4
  if (total === 0) {
    return 1
  }
  let dark = 0
  for (let offset = 0; offset < pixels.length; offset += 4) {
    if (isNearBlack(pixels, offset)) {
      dark += 1
    }
  }
  return dark / total
}

const percentile = (values: Float32Array, fraction: number): number => {
  const sorted = Float32Array.from(values)
  sorted.sort()
  const index = Math.min(sorted.length - 1, Math.max(0, Math.floor((sorted.length - 1) * fraction)))
  return sorted[index] ?? 0
}

const prepareFrame = (frame: ScreenFrame): PreparedFrame => {
  const detected = detectContentRect(frame)
  const { rect } = detected
  const pixelCount = rect.width * rect.height
  const lab = new Float32Array(pixelCount * 3)
  const chroma = new Float32Array(pixelCount)
  const { pixels, width } = frame

  let cursor = 0
  let sumL = 0
  for (let y = rect.y; y < rect.y + rect.height; y += 1) {
    for (let x = rect.x; x < rect.x + rect.width; x += 1) {
      const offset = (y * width + x) * 4
      const color = rgbToOklch({
        red: pixels[offset] ?? 0,
        green: pixels[offset + 1] ?? 0,
        blue: pixels[offset + 2] ?? 0,
      })
      const radians = (color.hue * Math.PI) / 180
      lab[cursor * 3] = color.lightness / 100
      lab[cursor * 3 + 1] = color.chroma * Math.cos(radians)
      lab[cursor * 3 + 2] = color.chroma * Math.sin(radians)
      chroma[cursor] = color.chroma
      sumL += color.lightness / 100
      cursor += 1
    }
  }

  return {
    width: frame.width,
    height: frame.height,
    rect,
    pixelCount,
    lab,
    chroma,
    meanL: sumL / Math.max(1, pixelCount),
    p90Chroma: percentile(chroma, 0.9),
    blackout: detected.rejected && fullDarkFraction(frame) >= BAR_COVERAGE,
  }
}

export { prepareFrame }
export type { ContentRect, PreparedFrame }
