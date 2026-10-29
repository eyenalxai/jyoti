import { String as Str } from "effect"
import { Show } from "solid-js"

import { palette } from "@/lib/theme"

interface SwatchProps {
  readonly hex: string
  readonly clipped: boolean
}

const Swatch = (props: SwatchProps) => (
  <box flexDirection="row" gap={1} height={1} alignItems="center">
    <text bg={palette.surfaceRaised} fg={props.hex}>
      {Str.repeat(12)("█")}
    </text>
    <text bg={palette.surfaceRaised} fg={palette.text}>
      {props.hex}
    </text>
    <Show when={props.clipped}>
      <text bg={palette.surfaceRaised} fg={palette.warning}>
        clipped to sRGB
      </text>
    </Show>
  </box>
)

export { Swatch }
