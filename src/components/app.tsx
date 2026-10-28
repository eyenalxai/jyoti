import { useKeyboard, useRenderer } from "@opentui/solid"

import { palette } from "@/lib/theme"

const App = () => {
  const renderer = useRenderer()

  useKeyboard((key) => {
    if (key.name === "q" && !key.ctrl && !key.meta && !key.option) {
      renderer.destroy()
    }
  })

  return (
    <box
      flexDirection="column"
      flexGrow={1}
      backgroundColor={palette.background}
      padding={1}
      gap={1}
    >
      <box flexDirection="column">
        <text fg={palette.accent}>jyoti</text>
        <text fg={palette.muted}>Xiaomi light control</text>
      </box>
      <box
        border
        borderStyle="rounded"
        borderColor={palette.border}
        title="bulbs"
        flexDirection="column"
        flexGrow={1}
        padding={1}
        gap={1}
      >
        <text fg={palette.text}>No bulbs configured yet.</text>
        <text fg={palette.muted}>
          Bare scaffold: the miIO client and the bulb controls come next.
        </text>
      </box>
      <text fg={palette.muted}>q quit · ctrl+c quit</text>
    </box>
  )
}

export { App }
