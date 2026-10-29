import type { JSX } from "solid-js"

import { useTerminalDimensions } from "@opentui/solid"
import { createMemo } from "solid-js"

import { palette } from "@/lib/theme"

interface ModalProps {
  readonly title: string
  readonly hint: string
  readonly width: number
  readonly height: number
  readonly children: JSX.Element
}

const Modal = (props: ModalProps) => {
  const dimensions = useTerminalDimensions()
  const width = createMemo(() => Math.max(20, Math.min(props.width, dimensions().width - 2)))
  const height = createMemo(() => Math.max(8, Math.min(props.height, dimensions().height - 2)))
  const left = createMemo(() => Math.max(0, Math.floor((dimensions().width - width()) / 2)))
  const top = createMemo(() => Math.max(0, Math.floor((dimensions().height - height()) / 2)))

  return (
    <box
      position="absolute"
      left={left()}
      top={top()}
      width={width()}
      height={height()}
      zIndex={20}
      flexDirection="column"
      gap={1}
      padding={1}
      border
      borderStyle="rounded"
      borderColor={palette.border}
      backgroundColor={palette.background}
      shouldFill
    >
      <box flexDirection="row" justifyContent="space-between" height={1}>
        <text bg={palette.surfaceRaised} fg={palette.text}>
          <strong>{props.title}</strong>
        </text>
        <text bg={palette.surfaceRaised} fg={palette.faint}>
          {props.hint}
        </text>
      </box>
      <box flexDirection="column" flexGrow={1} minHeight={0} shouldFill overflow="hidden">
        {props.children}
      </box>
    </box>
  )
}

export { Modal }
