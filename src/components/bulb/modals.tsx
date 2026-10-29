import { Show } from "solid-js"

import type { Bulb, BulbDraft } from "@/lib/bulbs/schema"

import { Color } from "@/components/bulb/color"
import { Form } from "@/components/bulb/form"
import { Help } from "@/components/bulb/help"
import { Remove } from "@/components/bulb/remove"
import { Modal } from "@/components/ui/modal"

type Screen = "dashboard" | "add" | "edit" | "remove" | "color" | "help"

interface ModalsProps {
  readonly screen: Screen
  readonly activeBulb: Bulb | null
  readonly colorInitial: string
  readonly busy: boolean
  readonly error: string | null
  readonly onSubmitAdd: (draft: BulbDraft) => void
  readonly onSubmitEdit: (draft: BulbDraft) => void
  readonly onConfirmRemove: () => void
  readonly onSubmitColor: (hex: string) => void
}

const Modals = (props: ModalsProps) => (
  <>
    <Show when={props.screen === "add"}>
      <Modal title="Add bulb" hint="esc cancel" width={58} height={16}>
        <Form
          mode="add"
          initial={null}
          inputWidth={41}
          busy={props.busy}
          error={props.error}
          onSubmit={props.onSubmitAdd}
        />
      </Modal>
    </Show>

    <Show when={props.screen === "edit" && props.activeBulb !== null}>
      <Modal title="Edit bulb" hint="esc cancel" width={58} height={16}>
        <Form
          mode="edit"
          initial={props.activeBulb}
          inputWidth={41}
          busy={props.busy}
          error={props.error}
          onSubmit={props.onSubmitEdit}
        />
      </Modal>
    </Show>

    <Show when={props.screen === "remove"}>
      <Show when={props.activeBulb}>
        {(bulb) => (
          <Modal title="Remove bulb" hint="y or n" width={50} height={12}>
            <Remove bulb={bulb()} busy={props.busy} error={props.error} />
          </Modal>
        )}
      </Show>
    </Show>

    <Show when={props.screen === "color" && props.activeBulb !== undefined}>
      <Modal title="Set color" hint="esc cancel" width={46} height={13}>
        <Color
          initial={props.colorInitial}
          inputWidth={20}
          busy={props.busy}
          error={props.error}
          onSubmit={props.onSubmitColor}
        />
      </Modal>
    </Show>

    <Show when={props.screen === "help"}>
      <Modal title="Keys" hint="esc close" width={58} height={17}>
        <Help />
      </Modal>
    </Show>
  </>
)

export { Modals, type Screen }
