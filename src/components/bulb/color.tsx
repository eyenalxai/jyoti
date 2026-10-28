import { String as Str } from "effect"
import { createMemo, createSignal, onCleanup, onMount } from "solid-js"

import { palette } from "@/lib/theme"
import { after, cancel } from "@/lib/timers"

interface ColorProps {
  readonly initial: string
  readonly inputWidth: number
  readonly busy: boolean
  readonly error: string | null
  readonly onSubmit: (hex: string) => void
}

const isValidHex = (value: string): boolean => /^#?[0-9a-fA-F]{6}$/u.test(Str.trim(value))

const normalize = (value: string): string => {
  const trimmed = Str.toUpperCase(Str.trim(value))
  return Str.startsWith("#")(trimmed) ? trimmed : `#${trimmed}`
}

const Color = (props: ColorProps) => {
  const [value, setValue] = createSignal("")
  const [issue, setIssue] = createSignal<string | null>(null)
  const [ready, setReady] = createSignal(false)

  onMount(() => {
    const timer = after(10, () => {
      setReady(true)
    })
    onCleanup(() => {
      cancel(timer)
    })
  })
  const effective = (): string =>
    Str.trim(value()).length === 0 ? props.initial : Str.trim(value())
  const preview = createMemo(() =>
    isValidHex(effective()) ? normalize(effective()) : palette.border,
  )

  const submit = () => {
    if (props.busy) {
      return
    }
    if (!isValidHex(effective())) {
      setIssue("Enter six hexadecimal digits, for example #FF8800.")
      return
    }
    setIssue(null)
    props.onSubmit(normalize(effective()))
  }

  return (
    <box flexDirection="column" gap={1} flexGrow={1}>
      <box flexDirection="row" gap={1} height={1}>
        <box width={8}>
          <text bg={palette.surfaceRaised} fg={palette.muted}>
            Hex
          </text>
        </box>
        <input
          width={props.inputWidth}
          value={value()}
          onInput={(next) => {
            setValue(next)
          }}
          onSubmit={submit}
          focused={ready()}
          maxLength={7}
          placeholder={props.initial}
          backgroundColor={palette.background}
          textColor={palette.text}
          focusedBackgroundColor={palette.background}
          cursorColor={palette.accent}
          placeholderColor={palette.faint}
        />
      </box>

      <box flexDirection="row" gap={1} height={1} alignItems="center">
        <text bg={palette.surfaceRaised} fg={preview()}>
          {Str.repeat(12)("█")}
        </text>
        <text bg={palette.surfaceRaised} fg={palette.faint}>
          {isValidHex(effective()) ? normalize(effective()) : "preview"}
        </text>
      </box>

      <box height={1}>
        <text
          fg={props.error === null && issue() === null ? palette.background : palette.danger}
          truncate
        >
          {props.error ?? issue() ?? " "}
        </text>
      </box>

      <box flexDirection="row" justifyContent="space-between" height={1}>
        <text bg={palette.surfaceRaised} fg={palette.faint}>
          esc cancel
        </text>
        <text bg={palette.surfaceRaised} fg={props.busy ? palette.faint : palette.accent}>
          {props.busy ? "applying…" : "enter apply"}
        </text>
      </box>
    </box>
  )
}

export { Color }
