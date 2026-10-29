import { palette } from "@/lib/theme"

interface StatusLineProps {
  readonly fallback: string
}

const StatusLine = (props: StatusLineProps) => (
  <box flexDirection="row" gap={1} height={1} flexGrow={1} minWidth={0}>
    <text fg={palette.muted} truncate wrapMode="none" flexGrow={1}>
      {props.fallback}
    </text>
  </box>
)

export { StatusLine }
