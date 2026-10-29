import type { ControlFocus } from "@/components/bulb/controls"
import type { Pane } from "@/components/bulb/pane"
import type { LightQuality } from "@/lib/bulbs/aggregate"

import { SYNC_OWNS_CONTROLS } from "@/lib/screen/sync/copy"

interface HintInput {
  readonly pane: Pane
  readonly focus: ControlFocus
  readonly quality: LightQuality
  readonly showingAll: boolean
  readonly hasBulbs: boolean
  readonly error: boolean
  readonly refreshing: boolean
  readonly syncActive: boolean
}

const lightHint = (quality: LightQuality, all: boolean): string => {
  if (quality === "white") {
    return all
      ? "←/→ warmth all · w white · c colour · ↑/↓"
      : "←/→ warmth · w white · c colour · ↑/↓"
  }
  if (quality === "color") {
    return all ? "enter or c picks all · w white · ↑/↓" : "enter or c picks · w white · ↑/↓"
  }
  return "w white · c colour · ↑/↓ control · tab bulbs"
}

const dashboardHint = (input: HintInput): string => {
  if (!input.hasBulbs) {
    return "no bulb selected"
  }
  if (input.error) {
    return "r retry"
  }
  if (input.refreshing) {
    return "refreshing…"
  }
  if (input.syncActive && input.pane === "controls" && input.focus !== "power") {
    return SYNC_OWNS_CONTROLS
  }
  if (input.pane === "bulbs") {
    return "↑/↓ select · tab controls"
  }
  if (input.focus === "power") {
    return input.showingAll
      ? "space toggle all · ↑/↓ control · tab bulbs"
      : "space toggle · ↑/↓ control · tab bulbs"
  }
  if (input.focus === "brightness") {
    return input.showingAll
      ? "←/→ set all · ↑/↓ control · tab bulbs"
      : "←/→ adjust · ↑/↓ control · tab bulbs"
  }
  return `${lightHint(input.quality, input.showingAll)} · tab bulbs`
}

interface GlobalHintInput {
  readonly syncActive: boolean
  readonly shellWidth: number
  readonly hasBulbs: boolean
  readonly canSelectAll: boolean
  readonly syncHint: string
}

const globalHint = (input: GlobalHintInput): string => {
  if (input.syncActive) {
    if (input.shellWidth < 90) {
      return `${input.syncHint} · ? help · q quit`
    }
    return `${input.syncHint} · r refresh · ? help · q quit`
  }
  if (!input.hasBulbs) {
    return `${input.syncHint} · ? help · q quit`
  }
  const all = input.canSelectAll ? "A all · " : ""
  if (input.shellWidth < 90) {
    return `${all}a add · ${input.syncHint} · ? help · q quit`
  }
  return `${all}a add · e edit · d remove · r refresh · ${input.syncHint} · ? help · q quit`
}

export { dashboardHint, globalHint }
