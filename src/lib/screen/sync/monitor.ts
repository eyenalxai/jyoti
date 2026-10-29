import { Array as Arr, Option } from "effect"

import type { Monitor } from "@/lib/screen/frame"

const pickDefaultMonitor = (monitors: readonly Monitor[]): Monitor | null => {
  const first = monitors[0]
  if (first === undefined) {
    return null
  }
  if (monitors.length === 1) {
    return first
  }
  let fallback = first
  for (const monitor of monitors) {
    if (monitor.focused === false) {
      fallback = monitor
      break
    }
  }
  return fallback
}

const selfCaptureFor = (monitor: Monitor, sole: boolean): boolean =>
  sole || monitor.focused !== false

const findMonitor = (monitors: readonly Monitor[], name: string): Monitor | null =>
  Option.getOrNull(Arr.findFirst(monitors, (monitor) => monitor.name === name))

const resolveMonitor = (monitors: readonly Monitor[], name: string | null): Monitor | null =>
  name === null ? pickDefaultMonitor(monitors) : findMonitor(monitors, name)

export { findMonitor, pickDefaultMonitor, resolveMonitor, selfCaptureFor }
