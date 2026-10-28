import type { RGBA } from "@opentui/core"

import { palette } from "@/lib/theme"

type NoticeKind = "info" | "success" | "error"

interface Notice {
  readonly kind: NoticeKind
  readonly message: string
}

interface StatusLineProps {
  readonly notice: Notice | null
  readonly fallback: string
}

const colorFor = (kind: NoticeKind): RGBA => {
  if (kind === "success") {
    return palette.success
  }
  if (kind === "error") {
    return palette.danger
  }
  return palette.muted
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

const StatusLine = (props: StatusLineProps) => {
  const notice = (): Notice | null => props.notice

  return (
    <box flexDirection="row" gap={1} height={1} flexGrow={1} minWidth={0}>
      <text fg={notice() === null ? palette.muted : colorFor(notice()?.kind ?? "info")}>
        {notice() === null ? props.fallback : glyphFor(notice()?.kind ?? "info")}
      </text>
      <text
        fg={notice() === null ? palette.muted : colorFor(notice()?.kind ?? "info")}
        truncate
        wrapMode="none"
        flexGrow={1}
      >
        {notice() === null ? "" : notice()?.message}
      </text>
    </box>
  )
}

export { StatusLine }
export type { Notice, NoticeKind }
