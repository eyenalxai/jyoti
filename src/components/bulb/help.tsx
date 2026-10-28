import { palette } from "@/lib/theme"

const KeyRow = (props: { readonly keys: string; readonly label: string }) => (
  <box flexDirection="row" gap={1} height={1} flexShrink={0}>
    <box width={14} flexShrink={0}>
      <text fg={palette.accent} bg={palette.surfaceRaised}>
        {props.keys}
      </text>
    </box>
    <text fg={palette.muted} bg={palette.surfaceRaised}>
      {props.label}
    </text>
  </box>
)

const Help = () => (
  <box flexDirection="column" gap={0}>
    <text fg={palette.faint} bg={palette.surfaceRaised} height={1} flexShrink={0}>
      bulbs
    </text>
    <KeyRow keys="↑ ↓  j k" label="Select a bulb or control" />
    <KeyRow keys="a  e  d" label="Add, edit or remove" />

    <text fg={palette.faint} bg={palette.surfaceRaised} height={1} flexShrink={0}>
      controls
    </text>
    <KeyRow keys="tab" label="Switch bulbs and controls" />
    <KeyRow keys="← →  h l" label="Brightness or warmth" />
    <KeyRow keys="space  enter" label="Toggle power or act" />
    <KeyRow keys="c" label="Set color" />

    <text fg={palette.faint} bg={palette.surfaceRaised} height={1} flexShrink={0}>
      app
    </text>
    <KeyRow keys="r  ?  q" label="Refresh, help, quit" />
  </box>
)

export { Help }
