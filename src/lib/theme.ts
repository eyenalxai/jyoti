import { RGBA } from "@opentui/core"

const palette = {
  background: RGBA.defaultBackground(),
  surfaceRaised: RGBA.defaultBackground(),
  border: RGBA.fromIndex(8),
  text: RGBA.defaultForeground(),
  muted: RGBA.fromIndex(8),
  faint: RGBA.fromIndex(8),
  accent: RGBA.fromIndex(11),
  success: RGBA.fromIndex(10),
  danger: RGBA.fromIndex(9),
  warning: RGBA.fromIndex(11),
} as const

export { palette }
