import type { Rgb } from "@/lib/bulbs/schema"

interface Hsb {
  readonly hue: number
  readonly saturation: number
  readonly value: number
}

const HUE_MAX = 360
const PERCENT_MAX = 100

const rgbToHsb = (rgb: Rgb): Hsb => {
  const red = rgb.red / 255
  const green = rgb.green / 255
  const blue = rgb.blue / 255
  const high = Math.max(red, green, blue)
  const low = Math.min(red, green, blue)
  const delta = high - low

  let hue = 0
  if (delta !== 0) {
    if (high === red) {
      hue = ((green - blue) / delta) % 6
    } else if (high === green) {
      hue = (blue - red) / delta + 2
    } else {
      hue = (red - green) / delta + 4
    }
    hue *= 60
    if (hue < 0) {
      hue += HUE_MAX
    }
  }

  return {
    hue,
    saturation: high === 0 ? 0 : (delta / high) * PERCENT_MAX,
    value: high * PERCENT_MAX,
  }
}

const hsbToRgb = (hsb: Hsb): Rgb => {
  const saturation = hsb.saturation / PERCENT_MAX
  const value = hsb.value / PERCENT_MAX
  const chroma = value * saturation
  const sector = (hsb.hue / 60) % 6
  const second = chroma * (1 - Math.abs((sector % 2) - 1))
  const match = value - chroma

  let red = 0
  let green = 0
  let blue = 0
  if (sector < 1) {
    red = chroma
    green = second
  } else if (sector < 2) {
    red = second
    green = chroma
  } else if (sector < 3) {
    green = chroma
    blue = second
  } else if (sector < 4) {
    green = second
    blue = chroma
  } else if (sector < 5) {
    red = second
    blue = chroma
  } else {
    red = chroma
    blue = second
  }

  return {
    red: Math.round((red + match) * 255),
    green: Math.round((green + match) * 255),
    blue: Math.round((blue + match) * 255),
  }
}

const withHue = (rgb: Rgb, hue: number): Rgb => hsbToRgb({ ...rgbToHsb(rgb), hue })

const withSaturation = (rgb: Rgb, saturation: number): Rgb =>
  hsbToRgb({ ...rgbToHsb(rgb), saturation })

export { HUE_MAX, PERCENT_MAX, hsbToRgb, rgbToHsb, withHue, withSaturation }
export type { Hsb }
