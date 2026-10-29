import { RGBA } from "@opentui/core"
import { createStore } from "solid-js/store"

interface Palette {
  readonly background: RGBA
  readonly surfaceRaised: RGBA
  readonly border: RGBA
  readonly text: RGBA
  readonly muted: RGBA
  readonly faint: RGBA
  readonly accent: RGBA
  readonly success: RGBA
  readonly danger: RGBA
  readonly warning: RGBA
}

const fallback: Palette = {
  background: RGBA.defaultBackground(),
  surfaceRaised: RGBA.defaultBackground(),
  border: RGBA.fromIndex(8),
  text: RGBA.defaultForeground(),
  muted: RGBA.fromIndex(8),
  faint: RGBA.fromIndex(8),
  accent: RGBA.fromIndex(11),
  success: RGBA.fromIndex(10),
  danger: RGBA.fromIndex(9),
  warning: RGBA.fromIndex(3),
}

const [palette, setPalette] = createStore<Palette>(fallback)

const blend = (foreground: RGBA, background: RGBA, weight: number): RGBA => {
  const [fr, fg, fb] = foreground.toInts()
  const [br, bg, bb] = background.toInts()
  const mix = (f: number, b: number) => Math.round(f * weight + b * (1 - weight))
  return RGBA.fromInts(mix(fr, br), mix(fg, bg), mix(fb, bb))
}

const channel = (value: number): number =>
  value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4

const luminance = (color: RGBA): number => {
  const [r, g, b] = color.toInts()
  return 0.2126 * channel(r / 255) + 0.7152 * channel(g / 255) + 0.0722 * channel(b / 255)
}

const contrast = (a: RGBA, b: RGBA): number => {
  const first = luminance(a)
  const second = luminance(b)
  return (Math.max(first, second) + 0.05) / (Math.min(first, second) + 0.05)
}

const toneAt = (foreground: RGBA, background: RGBA, target: number): RGBA => {
  if (contrast(foreground, background) <= target) {
    return foreground
  }
  let low = 0
  let high = 1
  for (let step = 0; step < 12; step += 1) {
    const weight = (low + high) / 2
    if (contrast(blend(foreground, background, weight), background) < target) {
      low = weight
    } else {
      high = weight
    }
  }
  return blend(foreground, background, high)
}

const applyTerminalPalette = (foreground: string | null, background: string | null): boolean => {
  if (foreground === null || background === null) {
    return false
  }
  const fg = RGBA.fromHex(foreground)
  const bg = RGBA.fromHex(background)
  setPalette({
    background: bg,
    surfaceRaised: blend(fg, bg, 0.06),
    border: toneAt(fg, bg, 2.5),
    text: fg,
    muted: toneAt(fg, bg, 8),
    faint: toneAt(fg, bg, 5),
    accent: RGBA.fromIndex(11),
    success: RGBA.fromIndex(10),
    danger: RGBA.fromIndex(9),
    warning: RGBA.fromIndex(3),
  })
  return true
}

export { applyTerminalPalette, palette }
