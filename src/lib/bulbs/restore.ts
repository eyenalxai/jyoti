import type { BulbState } from "@/lib/bulbs/schema"

type RestoreControl =
  | { readonly kind: "color"; readonly hex: string }
  | { readonly kind: "temperature"; readonly kelvin: number }

const restoreControl = (state: BulbState): RestoreControl | null => {
  if (state.mode === "temperature" && state.temperature !== null) {
    return { kind: "temperature", kelvin: state.temperature }
  }
  if (state.color !== null) {
    return { kind: "color", hex: state.color }
  }
  if (state.temperature !== null) {
    return { kind: "temperature", kelvin: state.temperature }
  }
  return null
}

export { restoreControl, type RestoreControl }
