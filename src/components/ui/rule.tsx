import { String as Str } from "effect"

import { palette } from "@/lib/theme"

const Rule = (props: { readonly width: number }) => (
  <text fg={palette.border} height={1} wrapMode="none">
    {Str.repeat(Math.max(0, props.width))("─")}
  </text>
)

export { Rule }
