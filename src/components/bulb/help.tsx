import { palette } from "@/lib/theme"

const KeyRow = (props: { readonly keys: string; readonly label: string }) => (
  <box flexDirection="row" gap={1} height={1} flexShrink={0}>
    <box width={12} flexShrink={0}>
      <text fg={palette.accent}>{props.keys}</text>
    </box>
    <text fg={palette.muted} truncate wrapMode="none">
      {props.label}
    </text>
  </box>
)

const Section = (props: { readonly label: string }) => (
  <text fg={palette.faint} height={1} flexShrink={0}>
    {props.label}
  </text>
)

const Help = () => (
  <box flexDirection="column" gap={0}>
    <Section label="bulbs" />
    <KeyRow keys="↑ ↓  j k" label="Select a bulb" />
    <KeyRow keys="A" label="All bulbs" />
    <KeyRow keys="a  e  d" label="Add, edit, remove" />

    <Section label="light" />
    <KeyRow keys="space" label="Power" />
    <KeyRow keys="← →  h l" label="Value, or switch the mode" />
    <KeyRow keys="w  c" label="White, or colour and pick" />
    <KeyRow keys="enter" label="Pick a colour, or apply" />

    <Section label="screen sync" />
    <KeyRow keys="s  x" label="Start or stop · pause" />
    <KeyRow keys="m  v" label="Monitor · capture preview" />

    <Section label="app" />
    <KeyRow keys="r  ?  q" label="Refresh, keys, quit" />
  </box>
)

export { Help }
