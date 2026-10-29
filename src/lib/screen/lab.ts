import type { Oklch } from "@/lib/color"

interface Lab {
  readonly l: number
  readonly a: number
  readonly b: number
}

const normalizeHue = (degrees: number): number => ((degrees % 360) + 360) % 360

const labFromOklch = (color: Oklch): Lab => {
  const radians = (color.hue * Math.PI) / 180
  return {
    l: color.lightness / 100,
    a: color.chroma * Math.cos(radians),
    b: color.chroma * Math.sin(radians),
  }
}

const oklchFromLab = (lab: Lab): Oklch => ({
  lightness: lab.l * 100,
  chroma: Math.hypot(lab.a, lab.b),
  hue: normalizeHue((Math.atan2(lab.b, lab.a) * 180) / Math.PI),
})

const deltaEOk = (a: Lab, b: Lab): number => Math.hypot(a.l - b.l, a.a - b.a, a.b - b.b)

export { deltaEOk, labFromOklch, normalizeHue, oklchFromLab }
export type { Lab }
