import { String as Str } from "effect"

import { palette } from "@/lib/theme"

interface SwatchProps {
  readonly hex: string
}

const Swatch = (props: SwatchProps) => (
  <box flexDirection="row" gap={1} height={1} alignItems="center">
    <text bg={palette.surfaceRaised} fg={props.hex}>
      {Str.repeat(12)("█")}
    </text>
    <text bg={palette.surfaceRaised} fg={palette.text}>
      {props.hex}
    </text>
    <text bg={palette.surfaceRaised} fg={palette.faint}>
      shown = sent
    </text>
  </box>
)

export { Swatch }
