import type { BorderCharacters, RGBA } from "@opentui/core"

import { useTerminalDimensions } from "@opentui/solid"
import { Show, createEffect, onCleanup } from "solid-js"

import { noticeDuration } from "@/lib/format"
import { palette } from "@/lib/theme"
import { after, cancel } from "@/lib/timers"

type NoticeKind = "info" | "success" | "error"

interface Notice {
  readonly kind: NoticeKind
  readonly message: string
}

interface ToastProps {
  readonly notice: Notice | null
  readonly pending: number
  readonly onDismiss: () => void
}

interface ToasterProps {
  readonly toasts: readonly Notice[]
  readonly onDismiss: () => void
}

const TOAST_MAX_WIDTH = 60
const splitBorder: BorderCharacters = {
  topLeft: "",
  topRight: "",
  bottomLeft: "",
  bottomRight: "",
  horizontal: " ",
  vertical: "┃",
  topT: "",
  bottomT: "",
  leftT: "",
  rightT: "",
  cross: "",
}

const colorFor = (kind: NoticeKind): RGBA => {
  if (kind === "success") {
    return palette.success
  }
  if (kind === "error") {
    return palette.danger
  }
  return palette.accent
}

const glyphFor = (kind: NoticeKind): string => {
  if (kind === "success") {
    return "✓"
  }
  if (kind === "error") {
    return "✗"
  }
  return "·"
}

const Toast = (props: ToastProps) => {
  const dimensions = useTerminalDimensions()

  createEffect(() => {
    const notice = props.notice
    if (notice === null) {
      return
    }
    const timer = after(noticeDuration(notice.message), props.onDismiss)
    onCleanup(() => {
      cancel(timer)
    })
  })

  return (
    <Show when={props.notice}>
      {(notice) => (
        <box
          position="absolute"
          top={1}
          right={2}
          zIndex={50}
          maxWidth={Math.max(20, Math.min(TOAST_MAX_WIDTH, dimensions().width - 6))}
          justifyContent="center"
          alignItems="flex-start"
          border={["left", "right"]}
          customBorderChars={splitBorder}
          borderColor={colorFor(notice().kind)}
          backgroundColor={palette.surfaceRaised}
          onMouseUp={props.onDismiss}
        >
          <box
            width="100%"
            paddingLeft={2}
            paddingRight={2}
            paddingTop={1}
            paddingBottom={1}
            backgroundColor={palette.surfaceRaised}
          >
            <box flexDirection="row" width="100%">
              <text fg={colorFor(notice().kind)}>{glyphFor(notice().kind)}</text>
              <text fg={palette.text} wrapMode="word" flexGrow={1} marginLeft={1}>
                {notice().message}
              </text>
              <text fg={palette.faint} marginLeft={2}>
                x
              </text>
            </box>
            <Show when={props.pending > 0}>
              <text fg={palette.faint} marginTop={1}>
                +{props.pending} more
              </text>
            </Show>
          </box>
        </box>
      )}
    </Show>
  )
}

const Toaster = (props: ToasterProps) => (
  <Toast
    notice={props.toasts[0] ?? null}
    pending={Math.max(0, props.toasts.length - 1)}
    onDismiss={props.onDismiss}
  />
)

export { Toast, Toaster }
export type { Notice, NoticeKind }
