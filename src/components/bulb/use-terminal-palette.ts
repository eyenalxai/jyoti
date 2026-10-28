import { useRenderer } from "@opentui/solid"
import { onMount } from "solid-js"

import { applyTerminalPalette } from "@/lib/theme"

const useTerminalPalette = (): void => {
  const renderer = useRenderer()

  onMount(() => {
    renderer
      .getPalette({ timeout: 150 })
      .then((colors) => applyTerminalPalette(colors.defaultForeground, colors.defaultBackground))
      .catch(() => {
        // Terminals and multiplexers that do not answer the OSC query keep the indexed fallback.
      })
  })
}

export { useTerminalPalette }
