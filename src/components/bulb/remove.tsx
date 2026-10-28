import type { Bulb } from "@/lib/bulbs/schema"

import { palette } from "@/lib/theme"

interface RemoveProps {
  readonly bulb: Bulb
  readonly busy: boolean
  readonly error: string | null
}

const Remove = (props: RemoveProps) => (
  <box flexDirection="column" gap={1} flexGrow={1}>
    <text bg={palette.surfaceRaised} fg={palette.text} wrapMode="word">
      Remove <strong>{props.bulb.name}</strong>?
    </text>
    <text bg={palette.surfaceRaised} fg={palette.muted}>
      Its token will be deleted from the keyring.
    </text>
    <text bg={palette.surfaceRaised} fg={palette.warning}>
      This cannot be undone.
    </text>
    <box height={1}>
      <text
        bg={palette.surfaceRaised}
        fg={props.error === null ? palette.background : palette.danger}
        truncate
      >
        {props.error ?? " "}
      </text>
    </box>
    <box flexDirection="row" justifyContent="space-between" height={1}>
      <text bg={palette.surfaceRaised} fg={palette.faint}>
        esc cancel
      </text>
      <text bg={palette.surfaceRaised} fg={props.busy ? palette.faint : palette.danger}>
        {props.busy ? "removing…" : "y remove"}
      </text>
    </box>
  </box>
)

export { Remove }
