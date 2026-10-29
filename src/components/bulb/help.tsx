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
    <KeyRow keys="A" label="Select all bulbs" />
    <KeyRow keys="a  e  d" label="Add, edit or remove" />

    <text fg={palette.faint} bg={palette.surfaceRaised} height={1} flexShrink={0}>
      controls
    </text>
    <KeyRow keys="tab" label="Switch bulbs and controls" />
    <KeyRow keys="← →  h l" label="Brightness or warmth" />
    <KeyRow keys="space  c" label="Toggle power or set color" />

    <text fg={palette.faint} bg={palette.surfaceRaised} height={1} flexShrink={0}>
      screen sync
    </text>
    <KeyRow keys="s" label="Start or stop sync" />
    <KeyRow keys="x" label="Pause or resume" />
    <KeyRow keys="m" label="Choose the captured monitor" />
    <KeyRow keys=" " label="sync owns colour, brightness and warmth" />
    <KeyRow keys=" " label="power untouched · 4000 K neutral · 1.5 Hz" />

    <text fg={palette.faint} bg={palette.surfaceRaised} height={1} flexShrink={0}>
      app
    </text>
    <KeyRow keys="r  ?  q" label="Refresh, help, quit" />
  </box>
)

export { Help }
