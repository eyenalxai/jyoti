export interface Rgb {
  readonly r: number
  readonly g: number
  readonly b: number
}

export interface Lab {
  readonly l: number
  readonly a: number
  readonly b: number
}

export interface Lch {
  readonly l: number
  readonly c: number
  readonly h: number
}

const srgbToLinearChannel = (value: number): number => {
  const c = value / 255
  return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4
}

const linearToSrgbChannel = (value: number): number => {
  const c = value <= 0.0031308 ? 12.92 * value : 1.055 * value ** (1 / 2.4) - 0.055
  return c * 255
}

export const srgbToLinear = (rgb: Rgb): readonly [number, number, number] => [
  srgbToLinearChannel(rgb.r),
  srgbToLinearChannel(rgb.g),
  srgbToLinearChannel(rgb.b),
]

export const linearToSrgb = (rgb: readonly [number, number, number]): Rgb => ({
  r: clampByte(linearToSrgbChannel(rgb[0])),
  g: clampByte(linearToSrgbChannel(rgb[1])),
  b: clampByte(linearToSrgbChannel(rgb[2])),
})

const clampByte = (value: number): number => Math.max(0, Math.min(255, Math.round(value)))

export const linearRgbToOklab = (r: number, g: number, b: number): Lab => {
  const l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b
  const m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b
  const s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b

  const lRoot = Math.cbrt(l)
  const mRoot = Math.cbrt(m)
  const sRoot = Math.cbrt(s)

  return {
    l: 0.2104542553 * lRoot + 0.793617785 * mRoot - 0.0040720468 * sRoot,
    a: 1.9779984951 * lRoot - 2.428592205 * mRoot + 0.4505937099 * sRoot,
    b: 0.0259040371 * lRoot + 0.7827717662 * mRoot - 0.808675766 * sRoot,
  }
}

export const oklabToLinearRgb = (lab: Lab): readonly [number, number, number] => {
  const lRoot = lab.l + 0.3963377774 * lab.a + 0.2158037573 * lab.b
  const mRoot = lab.l - 0.1055613458 * lab.a - 0.0638541728 * lab.b
  const sRoot = lab.l - 0.0894841775 * lab.a - 1.291485548 * lab.b

  const l = lRoot ** 3
  const m = mRoot ** 3
  const s = sRoot ** 3

  return [
    4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
  ]
}

export const rgbToOklab = (rgb: Rgb): Lab => {
  const [r, g, b] = srgbToLinear(rgb)
  return linearRgbToOklab(r, g, b)
}

export const oklabToRgb = (lab: Lab): Rgb => linearToSrgb(oklabToLinearRgb(lab))

export const oklabToOklch = (lab: Lab): Lch => ({
  l: lab.l,
  c: Math.hypot(lab.a, lab.b),
  h: normalizeHue((Math.atan2(lab.b, lab.a) * 180) / Math.PI),
})

export const oklchToOklab = (lch: Lch): Lab => {
  const radians = (lch.h * Math.PI) / 180
  return { l: lch.l, a: lch.c * Math.cos(radians), b: lch.c * Math.sin(radians) }
}

export const normalizeHue = (hue: number): number => ((hue % 360) + 360) % 360

export const hueDistance = (a: number, b: number): number => {
  const delta = Math.abs(normalizeHue(a) - normalizeHue(b))
  return Math.min(delta, 360 - delta)
}

export const deltaEOk = (a: Lab, b: Lab): number => Math.hypot(a.l - b.l, a.a - b.b, a.b - b.b)

export const inGamut = (lab: Lab): boolean => {
  const [r, g, b] = oklabToLinearRgb(lab)
  const epsilon = 1e-7
  return (
    r >= -epsilon &&
    r <= 1 + epsilon &&
    g >= -epsilon &&
    g <= 1 + epsilon &&
    b >= -epsilon &&
    b <= 1 + epsilon
  )
}

export interface GamutResult {
  readonly lch: Lch
  readonly rgb: Rgb
  readonly clamped: boolean
}

export const gamutMapOklch = (lch: Lch): GamutResult => {
  if (lch.l >= 1) {
    return { lch: { l: 1, c: 0, h: lch.h }, rgb: { r: 255, g: 255, b: 255 }, clamped: true }
  }
  if (lch.l <= 0) {
    return { lch: { l: 0, c: 0, h: lch.h }, rgb: { r: 0, g: 0, b: 0 }, clamped: true }
  }

  const candidate = oklchToOklab(lch)
  if (inGamut(candidate) && lch.c <= 0.5) {
    return { lch, rgb: oklabToRgb(candidate), clamped: false }
  }

  let low = 0
  let high = Math.min(lch.c, 0.5)
  for (let index = 0; index < 28; index += 1) {
    const mid = (low + high) / 2
    if (inGamut(oklchToOklab({ l: lch.l, c: mid, h: lch.h }))) {
      low = mid
    } else {
      high = mid
    }
  }
  return {
    lch: { l: lch.l, c: low, h: lch.h },
    rgb: oklabToRgb(oklchToOklab({ l: lch.l, c: low, h: lch.h })),
    clamped: true,
  }
}

export const toHex = (rgb: Rgb): string =>
  `#${[rgb.r, rgb.g, rgb.b]
    .map((channel) =>
      Math.max(0, Math.min(255, Math.round(channel)))
        .toString(16)
        .padStart(2, "0"),
    )
    .join("")}`

export const parseHex = (hex: string): Rgb => {
  const value = hex.replace("#", "")
  return {
    r: Number.parseInt(value.slice(0, 2), 16),
    g: Number.parseInt(value.slice(2, 4), 16),
    b: Number.parseInt(value.slice(4, 6), 16),
  }
}

/** Tanner Helland's blackbody approximation. Good enough for report swatches. */
export const kelvinToRgb = (kelvin: number): Rgb => {
  const temp = kelvin / 100
  const red = temp <= 66 ? 255 : 329.698727446 * (temp - 60) ** -0.1332047592
  const green =
    temp <= 66
      ? 99.4708025861 * Math.log(temp) - 161.1195681661
      : 288.1221695283 * (temp - 60) ** -0.0755148492
  const blue =
    temp >= 66 ? 255 : temp <= 19 ? 0 : 138.5177312231 * Math.log(temp - 10) - 305.0447927307
  return {
    r: clampByte(red),
    g: clampByte(green),
    b: clampByte(blue),
  }
}

export const kelvinToLab = (kelvin: number): Lab => rgbToOklab(kelvinToRgb(kelvin))

/** OKLab -> sRGB with no gamut mapping, for diagnostics. */
export const labToRgbUnclamped = (lab: Lab): readonly [number, number, number] => {
  const linear = oklabToLinearRgb(lab)
  return linear.map((channel) => linearToSrgbChannel(channel)) as unknown as readonly [
    number,
    number,
    number,
  ]
}

/** Perceptual chroma cap from JYO-19: C_max = 0.04 + 0.0009 * B, with B = L * 100. */
export const capChroma = (lch: Lch): Lch => {
  const maximum = 0.04 + 0.0009 * (lch.l * 100)
  return lch.c <= maximum ? lch : { ...lch, c: maximum }
}
