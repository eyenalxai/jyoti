import { palette } from "@/lib/theme"

type Mode = "oklch" | "rgb"

interface ModeTabsProps {
  readonly mode: Mode
}

const ModeTabs = (props: ModeTabsProps) => (
  <box flexDirection="row" justifyContent="space-between" height={1}>
    <box flexDirection="row" gap={2}>
      <text bg={palette.surfaceRaised} fg={props.mode === "oklch" ? palette.accent : palette.muted}>
        <strong>OKLCH</strong>
      </text>
      <text bg={palette.surfaceRaised} fg={props.mode === "rgb" ? palette.accent : palette.muted}>
        <strong>RGB</strong>
      </text>
    </box>
    <text bg={palette.surfaceRaised} fg={palette.faint}>
      tab switch
    </text>
  </box>
)

export { ModeTabs, type Mode }
