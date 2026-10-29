import type { Lab } from "./color"

import { linearRgbToOklab, srgbToLinear } from "./color"

export interface RawFrame {
  readonly width: number
  readonly height: number
  readonly data: Uint8Array
}

export interface ContentRect {
  readonly x: number
  readonly y: number
  readonly width: number
  readonly height: number
}

export interface PreparedFrame {
  readonly width: number
  readonly height: number
  readonly rect: ContentRect
  readonly pixelCount: number
  readonly lab: Float32Array
  readonly chroma: Float32Array
  readonly hue: Float32Array
  readonly luma: Float32Array
  readonly rgb: Uint8Array
  readonly meanL: number
  readonly meanChroma: number
  readonly meanHue: number
  readonly hueWeight: number
  readonly p90Chroma: number
  readonly p95Chroma: number
}

export const readRawFrame = async (
  path: string,
  width: number,
  height: number,
): Promise<RawFrame> => {
  const data = new Uint8Array(await Bun.file(path).arrayBuffer())
  if (data.length !== width * height * 4) {
    throw new Error(`${path} is ${data.length} bytes, expected ${width * height * 4}`)
  }
  return { width, height, data }
}

const isNearBlack = (data: Uint8Array, offset: number): boolean =>
  data[offset] <= 8 && data[offset + 1] <= 8 && data[offset + 2] <= 8

const darkFraction = (
  data: Uint8Array,
  width: number,
  height: number,
  axis: "row" | "column",
  index: number,
): number => {
  let dark = 0
  const count = axis === "row" ? width : height
  for (let cursor = 0; cursor < count; cursor += 1) {
    const x = axis === "row" ? cursor : index
    const y = axis === "row" ? index : cursor
    const offset = (y * width + x) * 4
    if (isNearBlack(data, offset)) {
      dark += 1
    }
  }
  return dark / count
}

export const detectContentRect = (frame: RawFrame): ContentRect => {
  const { width, height, data } = frame
  const barThreshold = 0.97
  let top = 0
  let bottom = height - 1
  let left = 0
  let right = width - 1

  while (top < height && darkFraction(data, width, height, "row", top) >= barThreshold) {
    top += 1
  }
  while (bottom >= top && darkFraction(data, width, height, "row", bottom) >= barThreshold) {
    bottom -= 1
  }
  while (left < width && darkFraction(data, width, height, "column", left) >= barThreshold) {
    left += 1
  }
  while (right >= left && darkFraction(data, width, height, "column", right) >= barThreshold) {
    right -= 1
  }

  const candidateWidth = right - left + 1
  const candidateHeight = bottom - top + 1
  if (
    candidateWidth <= 0 ||
    candidateHeight <= 0 ||
    candidateWidth * candidateHeight < width * height * 0.25
  ) {
    return { x: 0, y: 0, width, height }
  }
  return { x: left, y: top, width: candidateWidth, height: candidateHeight }
}

const percentile = (values: Float32Array, fraction: number): number => {
  const copy = Array.from(values)
  copy.sort((a, b) => a - b)
  const index = Math.min(copy.length - 1, Math.max(0, Math.floor((copy.length - 1) * fraction)))
  return copy[index] ?? 0
}

export const prepareFrame = (
  frame: RawFrame,
  rect: ContentRect = detectContentRect(frame),
): PreparedFrame => {
  const { width, data } = frame
  const pixelCount = rect.width * rect.height
  const lab = new Float32Array(pixelCount * 3)
  const chroma = new Float32Array(pixelCount)
  const hue = new Float32Array(pixelCount)
  const luma = new Float32Array(pixelCount)
  const rgb = new Uint8Array(pixelCount * 3)

  let cursor = 0
  let sumL = 0
  let sumChroma = 0
  let hueX = 0
  let hueY = 0
  let hueWeight = 0
  for (let y = rect.y; y < rect.y + rect.height; y += 1) {
    for (let x = rect.x; x < rect.x + rect.width; x += 1) {
      const offset = (y * width + x) * 4
      const r = data[offset] ?? 0
      const g = data[offset + 1] ?? 0
      const b = data[offset + 2] ?? 0
      const [linearR, linearG, linearB] = srgbToLinear({ r, g, b })
      const value: Lab = linearRgbToOklab(linearR, linearG, linearB)
      lab[cursor * 3] = value.l
      lab[cursor * 3 + 1] = value.a
      lab[cursor * 3 + 2] = value.b
      chroma[cursor] = Math.hypot(value.a, value.b)
      hue[cursor] = ((Math.atan2(value.b, value.a) * 180) / Math.PI + 360) % 360
      luma[cursor] = 0.2126 * linearR + 0.7152 * linearG + 0.0722 * linearB
      sumL += value.l
      sumChroma += chroma[cursor] ?? 0
      const pixelChroma = chroma[cursor] ?? 0
      if (pixelChroma > 0.005) {
        const radians = ((hue[cursor] ?? 0) * Math.PI) / 180
        hueX += pixelChroma * Math.cos(radians)
        hueY += pixelChroma * Math.sin(radians)
        hueWeight += pixelChroma
      }
      rgb[cursor * 3] = r
      rgb[cursor * 3 + 1] = g
      rgb[cursor * 3 + 2] = b
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
    hue,
    luma,
    rgb,
    meanL: sumL / Math.max(1, pixelCount),
    meanChroma: sumChroma / Math.max(1, pixelCount),
    meanHue: hueWeight > 0 ? ((Math.atan2(hueY, hueX) * 180) / Math.PI + 360) % 360 : 0,
    hueWeight,
    p90Chroma: percentile(chroma, 0.9),
    p95Chroma: percentile(chroma, 0.95),
  }
}

export const labAt = (frame: PreparedFrame, index: number): Lab => ({
  l: frame.lab[index * 3] ?? 0,
  a: frame.lab[index * 3 + 1] ?? 0,
  b: frame.lab[index * 3 + 2] ?? 0,
})
