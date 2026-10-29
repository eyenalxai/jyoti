import { palette } from "@/lib/theme"

type Mode = "hs" | "rgb"

interface ModeTabsProps {
  readonly mode: Mode
}

const ModeTabs = (props: ModeTabsProps) => (
  <box flexDirection="row" gap={1} height={1}>
    <text
      bg={props.mode === "hs" ? palette.accent : palette.background}
      fg={props.mode === "hs" ? palette.background : palette.muted}
    >
      {` Hue + Saturation `}
    </text>
    <text
      bg={props.mode === "rgb" ? palette.accent : palette.background}
      fg={props.mode === "rgb" ? palette.background : palette.muted}
    >
      {` RGB `}
    </text>
  </box>
)

export { ModeTabs, type Mode }
