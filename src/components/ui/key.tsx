import type { JSX } from "solid-js"

import { For } from "solid-js"

import { palette } from "@/lib/theme"

interface Hint {
  readonly keys: string
  readonly label: string
}

const Key = (props: { readonly children: JSX.Element }) => (
  <text fg={palette.accent} bg={palette.background}>
    [{props.children}]
  </text>
)

const KeyBar = (props: { readonly hints: readonly Hint[] }) => (
  <box
    flexDirection="row"
    flexWrap="wrap"
    justifyContent="center"
    columnGap={2}
    rowGap={0}
    width="100%"
    flexShrink={0}
  >
    <For each={props.hints}>
      {(hint) => (
        <text height={1} wrapMode="none">
          <span style={{ fg: palette.accent }}>{hint.keys}</span>
          <span style={{ fg: palette.faint }}> {hint.label}</span>
        </text>
      )}
    </For>
  </box>
)

export { Key, KeyBar, type Hint }
