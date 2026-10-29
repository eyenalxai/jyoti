import { palette } from "@/lib/theme"

type HelpContext = "dashboard" | "sync" | "color" | "monitors" | "capture" | "remove" | "form"

interface HelpProps {
  readonly context: HelpContext
  readonly syncActive: boolean
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

const DashboardHelp = (props: { readonly syncActive: boolean }) => (
  <box flexDirection="column" gap={0}>
    <Section label="navigation" />
    <KeyRow keys="↑ ↓  j k" label="Select a bulb, or move focus" />
    <KeyRow keys="tab" label="Switch the bulb list and controls" />
    <KeyRow keys="A" label="All bulbs" />

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
    <KeyRow keys="r  v  ?  q" label="Refresh, preview, keys, quit" />
  </box>
)

const SyncHelp = () => (
  <box flexDirection="column" gap={0}>
    <Section label="sync panel" />
    <KeyRow keys="↑ ↓" label="Move between rows" />
    <KeyRow keys="← →" label="Adjust the brightness gain" />
    <KeyRow keys="enter" label="Start, stop, pause or choose the monitor" />
    <KeyRow keys="esc" label="Close the panel" />
    <KeyRow keys="?" label="This help" />
  </box>
)

const ColorHelp = () => (
  <box flexDirection="column" gap={0}>
    <Section label="colour picker" />
    <KeyRow keys="↑ ↓" label="Move between fields" />
    <KeyRow keys="← →" label="Adjust the focused slider" />
    <KeyRow keys="tab" label="Hue/saturation or RGB" />
    <KeyRow keys="enter" label="Keep the colour and close" />
    <KeyRow keys="esc" label="Revert to the opening colour" />
    <KeyRow keys="?" label="This help" />
  </box>
)

const MonitorsHelp = () => (
  <box flexDirection="column" gap={0}>
    <Section label="monitor picker" />
    <KeyRow keys="↑ ↓" label="Move between monitors" />
    <KeyRow keys="enter" label="Choose the monitor" />
    <KeyRow keys="esc" label="Cancel" />
    <KeyRow keys="?" label="This help" />
  </box>
)

const CaptureHelp = () => (
  <box flexDirection="column" gap={0}>
    <Section label="capture preview" />
    <KeyRow keys="↑ ↓" label="Move between monitors" />
    <KeyRow keys="enter" label="Start or stop the capture" />
    <KeyRow keys="esc" label="Close the preview" />
    <KeyRow keys="?" label="This help" />
  </box>
)

const RemoveHelp = () => (
  <box flexDirection="column" gap={0}>
    <Section label="remove bulb" />
    <KeyRow keys="y" label="Remove the bulb" />
    <KeyRow keys="n  esc" label="Cancel" />
  </box>
)

const FormHelp = () => (
  <box flexDirection="column" gap={0}>
    <Section label="form" />
    <KeyRow keys="tab" label="Move between fields" />
    <KeyRow keys="enter" label="Submit" />
    <KeyRow keys="esc" label="Cancel" />
  </box>
)

const Help = (props: HelpProps) => {
  if (props.context === "sync") {
    return <SyncHelp />
  }
  if (props.context === "color") {
    return <ColorHelp />
  }
  if (props.context === "monitors") {
    return <MonitorsHelp />
  }
  if (props.context === "capture") {
    return <CaptureHelp />
  }
  if (props.context === "remove") {
    return <RemoveHelp />
  }
  if (props.context === "form") {
    return <FormHelp />
  }
  return <DashboardHelp syncActive={props.syncActive} />
}

export { Help }
export type { HelpContext }
