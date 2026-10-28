import { useKeyboard } from "@opentui/solid"
import { Array as Arr, Option, String as Str } from "effect"
import { createSignal, onCleanup, onMount } from "solid-js"

import type { Bulb, BulbDraft, BulbField } from "@/lib/bulbs/schema"

import { Key } from "@/components/ui/key"
import { validateDraft } from "@/lib/bulbs/schema"
import { palette } from "@/lib/theme"
import { after, cancel } from "@/lib/timers"

interface FormProps {
  readonly mode: "add" | "edit"
  readonly initial: Bulb | null
  readonly inputWidth: number
  readonly busy: boolean
  readonly error: string | null
  readonly onSubmit: (draft: BulbDraft) => void
}

const fieldIndex: Readonly<Partial<Record<BulbField, number>>> = {
  name: 0,
  address: 1,
  model: 2,
  token: 3,
}

const Form = (props: FormProps) => {
  const [name, setName] = createSignal(props.initial?.name ?? "")
  const [address, setAddress] = createSignal(props.initial?.address ?? "")
  const [model, setModel] = createSignal(props.initial?.model ?? "")
  const [token, setToken] = createSignal("")
  const [focus, setFocus] = createSignal(0)
  const [ready, setReady] = createSignal(false)

  onMount(() => {
    const timer = after(10, () => {
      setReady(true)
    })
    onCleanup(() => {
      cancel(timer)
    })
  })
  const [issues, setIssues] = createSignal<
    readonly { readonly field: BulbField; readonly message: string }[]
  >([])

  const issueFor = (field: BulbField): string | undefined =>
    Option.getOrUndefined(Arr.findFirst(issues(), (issue) => issue.field === field))?.message

  const clearIssue = (field: BulbField) => {
    setIssues((previous) => Arr.filter(previous, (issue) => issue.field !== field))
  }

  const hintFor = (field: BulbField, fallback: string): string => {
    const issue = issueFor(field)
    if (issue !== undefined) {
      return issue
    }
    return fallback
  }

  useKeyboard((key) => {
    if (key.eventType === "release") {
      return
    }
    if (key.name !== "tab") {
      return
    }
    setFocus((current) => (current + (key.shift ? 3 : 1)) % 4)
  })

  const submit = () => {
    if (props.busy) {
      return
    }
    const draft = {
      name: Str.trim(name()),
      address: Str.trim(address()),
      model: Str.trim(model()),
      token: Str.trim(token()),
    }
    const found = validateDraft(draft, { tokenRequired: props.mode === "add" })
    if (found.length > 0) {
      setIssues(found)
      const first = found[0]
      if (first !== undefined) {
        setFocus(fieldIndex[first.field] ?? 0)
      }
      return
    }
    setIssues([])
    props.onSubmit(draft)
  }

  const input = (index: number, width: number) => ({
    width,
    focused: ready() && focus() === index,
    backgroundColor: palette.background,
    textColor: palette.text,
    focusedBackgroundColor: palette.background,
    cursorColor: palette.accent,
    placeholderColor: palette.faint,
  })

  return (
    <box flexDirection="column" flexGrow={1} minHeight={0}>
      <box flexDirection="column">
        <box height={2} flexDirection="column">
          <box flexDirection="row" gap={1} height={1}>
            <box width={12}>
              <text bg={palette.surfaceRaised} fg={focus() === 0 ? palette.text : palette.muted}>
                Name
              </text>
            </box>
            <input
              {...input(0, props.inputWidth)}
              value={name()}
              onInput={(value) => {
                setName(value)
                clearIssue("name")
              }}
              onSubmit={submit}
              placeholder="Desk lamp"
              maxLength={60}
            />
          </box>
          <box paddingLeft={13} height={1}>
            <text
              bg={palette.surfaceRaised}
              fg={issueFor("name") === undefined ? palette.faint : palette.danger}
            >
              {hintFor("name", "A name you will recognise")}
            </text>
          </box>
        </box>

        <box height={2} flexDirection="column">
          <box flexDirection="row" gap={1} height={1}>
            <box width={12}>
              <text bg={palette.surfaceRaised} fg={focus() === 1 ? palette.text : palette.muted}>
                IP address
              </text>
            </box>
            <input
              {...input(1, props.inputWidth)}
              value={address()}
              onInput={(value) => {
                setAddress(value)
                clearIssue("address")
              }}
              onSubmit={submit}
              placeholder="192.168.1.42"
              maxLength={45}
            />
          </box>
          <box paddingLeft={13} height={1}>
            <text
              bg={palette.surfaceRaised}
              fg={issueFor("address") === undefined ? palette.faint : palette.danger}
            >
              {hintFor("address", "IPv4 or IPv6 address")}
            </text>
          </box>
        </box>

        <box height={2} flexDirection="column">
          <box flexDirection="row" gap={1} height={1}>
            <box width={12}>
              <text bg={palette.surfaceRaised} fg={focus() === 2 ? palette.text : palette.muted}>
                Model
              </text>
            </box>
            <input
              {...input(2, props.inputWidth)}
              value={model()}
              onInput={(value) => {
                setModel(value)
                clearIssue("model")
              }}
              onSubmit={submit}
              placeholder="optional"
              maxLength={60}
            />
          </box>
          <box paddingLeft={13} height={1}>
            <text
              bg={palette.surfaceRaised}
              fg={issueFor("model") === undefined ? palette.faint : palette.danger}
            >
              {hintFor("model", "Optional, for your own reference")}
            </text>
          </box>
        </box>

        <box height={2} flexDirection="column">
          <box flexDirection="row" gap={1} height={1}>
            <box width={12}>
              <text bg={palette.surfaceRaised} fg={focus() === 3 ? palette.text : palette.muted}>
                Token
              </text>
            </box>
            <input
              {...input(3, props.inputWidth)}
              value={token()}
              onInput={(value) => {
                setToken(value)
                clearIssue("token")
              }}
              onSubmit={submit}
              placeholder={props.mode === "add" ? "32 hex characters" : "leave blank to keep"}
              maxLength={32}
            />
          </box>
          <box paddingLeft={13} height={1}>
            <text
              bg={palette.surfaceRaised}
              fg={issueFor("token") === undefined ? palette.faint : palette.danger}
            >
              {hintFor("token", "Saved in the OS keyring")}
            </text>
          </box>
        </box>
      </box>

      <box height={1}>
        <text
          bg={palette.surfaceRaised}
          fg={props.error === null ? palette.background : palette.danger}
          truncate
        >
          {props.error ?? " "}
        </text>
      </box>

      <box flexDirection="row" justifyContent="space-between" height={1}>
        <text bg={palette.surfaceRaised} fg={palette.faint}>
          tab next · esc cancel
        </text>
        <box flexDirection="row" gap={1} alignItems="center">
          <text bg={palette.surfaceRaised} fg={props.busy ? palette.faint : palette.accent}>
            {props.busy ? "saving…" : "save"}
          </text>
          <Key>enter</Key>
        </box>
      </box>
    </box>
  )
}

export { Form }
