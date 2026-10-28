import type { JSX } from "solid-js"

import { palette } from "@/lib/theme"

const Key = (props: { readonly children: JSX.Element }) => (
  <text fg={palette.accent} bg={palette.background}>
    [{props.children}]
  </text>
)

export { Key }
