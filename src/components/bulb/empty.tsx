import { Key } from "@/components/ui/key"
import { palette } from "@/lib/theme"

const Empty = () => (
  <box flexGrow={1} flexDirection="column" alignItems="center" justifyContent="center" gap={1}>
    <text fg={palette.accent}>◉</text>
    <text fg={palette.text}>
      <strong>No bulbs yet</strong>
    </text>
    <text fg={palette.muted}>Add your first bulb to start controlling it.</text>
    <box flexDirection="row" gap={1} alignItems="center">
      <Key>a</Key>
      <text fg={palette.muted}>add a bulb</text>
    </box>
    <text fg={palette.faint}>Tokens stay in the OS keyring.</text>
  </box>
)

export { Empty }
