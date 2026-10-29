import type { ColorHex, Rgb } from "@/lib/bulbs/schema"

import { formatColor, packColor, parseColor } from "@/lib/bulbs/schema"
import { clamp } from "@/lib/format"

interface Oklch {
  readonly lightness: number
  readonly chroma: number
  readonly hue: number
}

interface ChannelRange {
  readonly min: number
  readonly max: number
  readonly step: number
}

interface SrgbColor {
  readonly rgb: Rgb
  readonly hex: ColorHex
  readonly clipped: boolean
}

interface LinearRgb {
  readonly red: number
  readonly green: number
  readonly blue: number
}

const LIGHTNESS_RANGE: ChannelRange = { min: 0, max: 100, step: 1 }
const CHROMA_RANGE: ChannelRange = { min: 0, max: 0.37, step: 0.01 }
const HUE_RANGE: ChannelRange = { min: 0, max: 360, step: 1 }

const GAMUT_EPSILON = 1 / 100_000
const FIT_STEPS = 18

const linearFromOklch = (color: Oklch): LinearRgb => {
  const lightness = color.lightness / LIGHTNESS_RANGE.max
  const radians = (color.hue * Math.PI) / 180
  const a = color.chroma * Math.cos(radians)
  const b = color.chroma * Math.sin(radians)
  const l = (lightness + 0.3963377774 * a + 0.2158037573 * b) ** 3
  const m = (lightness - 0.1055613458 * a - 0.0638541728 * b) ** 3
  const s = (lightness - 0.0894841775 * a - 1.291485548 * b) ** 3
  return {
    red: 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    green: -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    blue: -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
  }
}

const inGamut = (rgb: LinearRgb): boolean =>
  rgb.red >= -GAMUT_EPSILON &&
  rgb.red <= 1 + GAMUT_EPSILON &&
  rgb.green >= -GAMUT_EPSILON &&
  rgb.green <= 1 + GAMUT_EPSILON &&
  rgb.blue >= -GAMUT_EPSILON &&
  rgb.blue <= 1 + GAMUT_EPSILON

const fitToGamut = (color: Oklch): { readonly color: Oklch; readonly clipped: boolean } => {
  const bounded: Oklch = {
    lightness: clamp(color.lightness, LIGHTNESS_RANGE.min, LIGHTNESS_RANGE.max),
    chroma: clamp(color.chroma, CHROMA_RANGE.min, CHROMA_RANGE.max),
    hue: clamp(color.hue, HUE_RANGE.min, HUE_RANGE.max),
  }
  if (inGamut(linearFromOklch(bounded))) {
    return { color: bounded, clipped: false }
  }

  let low = CHROMA_RANGE.min
  let high = bounded.chroma
  for (let step = 0; step < FIT_STEPS; step += 1) {
    const middle = (low + high) / 2
    if (inGamut(linearFromOklch({ ...bounded, chroma: middle }))) {
      low = middle
    } else {
      high = middle
    }
  }
  return { color: { ...bounded, chroma: low }, clipped: low < bounded.chroma }
}

const encodeChannel = (value: number): number => {
  const bounded = clamp(value, 0, 1)
  const encoded = bounded <= 0.0031308 ? bounded * 12.92 : 1.055 * bounded ** (1 / 2.4) - 0.055
  return Math.round(encoded * 255)
}

const decodeChannel = (value: number): number => {
  const channel = value / 255
  return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4
}

const oklchToSrgb = (color: Oklch): SrgbColor => {
  const fitted = fitToGamut(color)
  const linear = linearFromOklch(fitted.color)
  const rgb: Rgb = {
    red: encodeChannel(linear.red),
    green: encodeChannel(linear.green),
    blue: encodeChannel(linear.blue),
  }
  return { rgb, hex: formatColor(packColor(rgb)), clipped: fitted.clipped }
}

const rgbToOklch = (rgb: Rgb): Oklch => {
  const red = decodeChannel(rgb.red)
  const green = decodeChannel(rgb.green)
  const blue = decodeChannel(rgb.blue)
  const l = Math.cbrt(0.4122214708 * red + 0.5363325363 * green + 0.0514459929 * blue)
  const m = Math.cbrt(0.2119034982 * red + 0.6806995451 * green + 0.1073969566 * blue)
  const s = Math.cbrt(0.0883024619 * red + 0.2817188376 * green + 0.6299787005 * blue)
  const lightness = 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s
  const a = 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s
  const b = 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s
  const chroma = Math.hypot(a, b)
  const hue = chroma < 1 / 10_000 ? 0 : (Math.atan2(b, a) * 180) / Math.PI
  return {
    lightness: clamp(lightness * LIGHTNESS_RANGE.max, LIGHTNESS_RANGE.min, LIGHTNESS_RANGE.max),
    chroma: clamp(chroma, CHROMA_RANGE.min, CHROMA_RANGE.max),
    hue: hue < 0 ? hue + HUE_RANGE.max : hue,
  }
}

const hexToOklch = (hex: string): Oklch | null => {
  const rgb = parseColor(hex)
  return rgb === null ? null : rgbToOklch(rgb)
}

export { CHROMA_RANGE, HUE_RANGE, LIGHTNESS_RANGE, hexToOklch, oklchToSrgb, rgbToOklch }
export type { ChannelRange, Oklch, SrgbColor }
