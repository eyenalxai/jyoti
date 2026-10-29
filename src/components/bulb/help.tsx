import { SYNC_OWNS_CONTROLS } from "@/lib/screen/sync/copy"
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

const NoteRow = (props: { readonly label: string }) => (
  <box flexDirection="row" gap={1} height={1} flexShrink={0}>
    <box width={14} flexShrink={0} />
    <text fg={palette.faint} bg={palette.surfaceRaised}>
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
    <KeyRow keys="A  a  e  d" label="All bulbs, add, edit or remove" />

    <text fg={palette.faint} bg={palette.surfaceRaised} height={1} flexShrink={0}>
      controls
    </text>
    <KeyRow keys="tab" label="Switch bulbs and controls" />
    <KeyRow keys="← →  h l" label="Brightness or warmth" />
    <KeyRow keys="space  enter" label="Toggle power or act" />
    <KeyRow keys="w  c" label="White light or set colour" />

    <text fg={palette.faint} bg={palette.surfaceRaised} height={1} flexShrink={0}>
      screen sync
    </text>
    <KeyRow keys="s  x" label="Start or stop (restores) · pause" />
    <KeyRow keys="m  v" label="Monitor · capture preview (dev)" />
    <NoteRow label={SYNC_OWNS_CONTROLS} />
    <NoteRow label="8–100% · ±8/tick · flash hold 2 s" />
    <NoteRow label="power untouched · 5 s lost" />

    <text fg={palette.faint} bg={palette.surfaceRaised} height={1} flexShrink={0}>
      app
    </text>
    <KeyRow keys="r  ?  q" label="Refresh, help, quit" />
  </box>
)

export { Help }
