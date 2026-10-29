import { palette } from "@/lib/theme"

interface HelpProps {
  readonly syncActive: boolean
  readonly colorOpen: boolean
}

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

const Help = (props: HelpProps) => {
  if (props.colorOpen) {
    return (
      <box flexDirection="column" gap={0}>
        <Section label="colour picker" />
        <KeyRow keys="↑ ↓" label="Move between fields" />
        <KeyRow keys="← →" label="Adjust the focused slider" />
        <KeyRow keys="tab" label="Hue/saturation or RGB" />
        <KeyRow keys="enter" label="Keep the colour and close" />
        <KeyRow keys="esc" label="Revert to the opening colour" />

        <Section label="app" />
        <KeyRow keys="q" label="Quit" />
      </box>
    )
  }

  return (
    <box flexDirection="column" gap={0}>
      <Section label="navigation" />
      <KeyRow keys="↑ ↓  j k" label="Select a bulb, or move focus" />
      <KeyRow keys="tab" label="Switch the bulb list and controls" />
      {props.syncActive ? null : <KeyRow keys="A" label="All bulbs" />}

      {props.syncActive ? null : (
        <>
          <Section label="bulbs" />
          <KeyRow keys="a  e  d" label="Add, edit, remove" />
        </>
      )}

      <Section label="light" />
      <KeyRow keys="space" label="Power" />
      {props.syncActive ? null : (
        <>
          <KeyRow keys="← →  h l" label="Value, or switch the mode" />
          <KeyRow keys="w  c" label="White, or colour and pick" />
          <KeyRow keys="enter" label="Pick a colour, or apply" />
        </>
      )}

      <Section label="screen sync" />
      <KeyRow keys="s" label="Open the Sync panel" />
      {props.syncActive ? (
        <KeyRow keys="—" label="Sync owns colour, brightness and temperature" />
      ) : null}

      <Section label="app" />
      <KeyRow keys="r  ?  q" label="Refresh, keys, quit" />
    </box>
  )
}

export { Help }
